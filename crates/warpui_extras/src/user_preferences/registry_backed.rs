use std::io;
use std::sync::Mutex;

/// Store user preferences in the Windows Registry.
/// Modeled after https://github.com/neovide/neovide/blob/main/src/windows_utils.rs .
use super::UserPreferences;
use windows_registry::{Key, CURRENT_USER};
use windows_result::HRESULT;

pub struct RegistryBackedPreferences {
    app_key_path: String,
    /// 缓存 `HKCU\Software\OpenWarp\<channel>` 注册表 Key 句柄。
    ///
    /// OpenWarp 启动时会顺序调用 ~100 个 setting 的 `read_value`,
    /// 每次都走 `CURRENT_USER.create(...)` 打开/创建 Key 是 ~3ms 的同步系统调用,
    /// 累计 300ms+(占冷启动 `READ_USER_DEFAULTS_AND_INITIALIZE_SETTINGS` 阶段大头)。
    /// 这里把第一次成功打开的 Key 缓存下来,后续读直接复用,省掉 N-1 次系统调用。
    ///
    /// 用 `Mutex<Option<Key>>` 而不是 `OnceLock`,因为 `windows_registry::Key`
    /// 没实现 `Clone`,需要可变锁来 `replace`/`take`;同时 `read_value` 接口是
    /// `&self`,无法用 `RefCell`(需要 Sync)。
    cached_key: Mutex<Option<Key>>,
}

static WARP_REGISTRY_BASE_PATH: &str = "Software\\OpenWarp\\";
pub const KEY_NOT_FOUND_ERR: HRESULT = HRESULT::from_win32(0x80070002);

impl RegistryBackedPreferences {
    /// Construct a separate registry path for each channel (stable, dev, local, etc.)
    pub fn new(app_name: &str) -> Self {
        let app_key_path = WARP_REGISTRY_BASE_PATH.to_owned() + app_name;
        // 启动时就预热 Key,让第一次 setting 读取也避开同步系统调用。
        // 预热失败不为错:`with_warp_registry` 会在需要时重试。
        let initial_key = CURRENT_USER
            .create(app_key_path.clone())
            .inspect_err(|e| {
                log::warn!("warp registry key prewarm failed (will retry on first access): {e:#}");
            })
            .ok();
        Self {
            app_key_path,
            cached_key: Mutex::new(initial_key),
        }
    }

    /// 用回调操作缓存的 Warp 注册表 Key。第一次会 `CURRENT_USER.create(...)`,
    /// 后续直接复用。如果 Key 锁中毒(之前 panic),fallback 到一次性 create
    /// 而不缓存 —— 行为退化但不会进一步 panic。
    fn with_warp_registry<R>(
        &self,
        f: impl FnOnce(&Key) -> Result<R, super::Error>,
    ) -> Result<R, super::Error> {
        let mut guard = match self.cached_key.lock() {
            Ok(g) => g,
            // Mutex 中毒:走一次性 create 路径,不缓存,行为等价原版。
            Err(_) => {
                let key = CURRENT_USER
                    .create(self.app_key_path.clone())
                    .map_err(|e| {
                        log::error!("unable to access Warp app key in Windows Registry: {e:#}");
                        super::Error::IoError(io::Error::from(e))
                    })?;
                return f(&key);
            }
        };

        if guard.is_none() {
            let key = CURRENT_USER
                .create(self.app_key_path.clone())
                .map_err(|e| {
                    log::error!("unable to access Warp app key in Windows Registry: {e:#}");
                    super::Error::IoError(io::Error::from(e))
                })?;
            *guard = Some(key);
        }

        // 此时 guard 必然 Some;unwrap 安全。
        f(guard.as_ref().expect("cached_key must be Some after init"))
    }
}

impl UserPreferences for RegistryBackedPreferences {
    fn read_value(&self, name: &str) -> Result<Option<String>, super::Error> {
        self.with_warp_registry(|key| Ok(key.get_string(name).ok()))
    }

    fn write_value(&self, key: &str, value: String) -> Result<(), super::Error> {
        self.with_warp_registry(|reg| {
            reg.set_string(key, value.as_str())
                .map_err(|e| super::Error::from(io::Error::from(e)))
        })
    }

    fn remove_value(&self, key: &str) -> Result<(), super::Error> {
        self.with_warp_registry(|reg| match reg.remove_value(key) {
            Ok(_) => Ok(()),
            // If the key doesn't exist, then treat removal of that nonexistent key as a success.
            Err(e) if e.code() == KEY_NOT_FOUND_ERR => Ok(()),
            Err(e) => Err(super::Error::from(io::Error::from(e))),
        })
    }
}
