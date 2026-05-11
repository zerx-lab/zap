use itertools::Itertools as _;
use std::os::windows::ffi::OsStrExt as _;
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

// Re-export a couple winit types and modules as the concrete implementations
// for Windows.
pub use crate::windowing::winit::app::App;

pub(crate) static DXC_PATH: std::sync::OnceLock<Option<DXCPath>> = std::sync::OnceLock::new();

/// Path to the DXC DLLs to be used to compile DirectX shaders using DXC.
/// See https://github.com/microsoft/DirectXShaderCompiler.
#[derive(Debug)]
pub struct DXCPath {
    pub dxc_path: String,
    pub dxil_path: String,
}

pub trait AppBuilderExt {
    /// Set the AppUserModel ID, which Windows uses to attribute notifications to
    /// our correct application.
    fn set_app_user_model_id(&mut self, app_id: String);

    /// Use DXC (the newer DirectX Shader Compiler) to compile DirectX shaders.
    /// Using DXC requires the dlls within [`DXCPath`] to be available and shipped
    /// alongside the application.=
    fn use_dxc_for_directx_shader_compilation(&mut self, dxc_path: DXCPath);
}

impl AppBuilderExt for super::AppBuilder {
    fn set_app_user_model_id(&mut self, app_id: String) {
        // 先把 AUMID 注册到 HKCU\Software\Classes\AppUserModelId\<aumid>,
        // 这样即使没有 Start Menu 快捷方式(`cargo run` 开发模式 / 解压版),
        // Windows ToastNotificationManager 也能找到该 AUMID,Toast 才会真正弹出。
        // 否则 `Toast::show()` 会被系统层静默吞掉,API 不报错。
        // 参考: https://learn.microsoft.com/en-us/windows/apps/design/shell/tiles-and-notifications/send-local-toast-other-apps
        if let Err(err) = register_aumid_in_registry(&app_id) {
            log::warn!("Unable to register Windows AppUserModel ID in registry: {err:?}");
        }

        let set_id = unsafe { set_app_user_model_id(app_id) };
        if let Err(err) = set_id {
            log::error!("Unable to set Windows AppUserModel ID: {err:?}");
        }
    }

    fn use_dxc_for_directx_shader_compilation(&mut self, dxc_path: DXCPath) {
        if let Err(e) = DXC_PATH.set(Some(dxc_path)) {
            log::warn!("Failed to set DXC path {e:?}");
        }
    }
}

unsafe fn set_app_user_model_id(app_id: String) -> Result<(), windows::core::Error> {
    let wide_string = std::ffi::OsStr::new(&app_id)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect_vec();
    windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(windows::core::PCWSTR(
        wide_string.as_ptr(),
    ))
}

/// 把 AUMID 注册到 `HKCU\Software\Classes\AppUserModelId\<aumid>`,
/// 这是 Windows 10/11 「unpackaged app」发送本地 toast 的官方注册路径。
///
/// `DisplayName` 决定 toast 上方显示的来源名;`IconBackgroundColor` 让
/// Windows 用更干净的纯色背景替代默认灰底。Icon 暂不写(需要绝对路径,
/// 且在 `cargo run` 与正式安装两种场景下路径不一样,留给 installer 处理)。
fn register_aumid_in_registry(app_id: &str) -> std::io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let subkey = format!("Software\\Classes\\AppUserModelId\\{app_id}");
    let (key, _) = hkcu.create_subkey(&subkey)?;

    // 从 AUMID 末段推导一个体面的展示名(e.g. dev.openwarp.OpenWarp → OpenWarp)。
    let display_name = app_id.rsplit('.').next().unwrap_or(app_id);
    key.set_value("DisplayName", &display_name.to_string())?;
    Ok(())
}
