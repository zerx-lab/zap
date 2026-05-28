#![allow(deprecated)]

use command::{blocking, r#async::Command};
use futures::StreamExt;
use futures_lite::future;
use instant::Instant;
use std::{
    env,
    ffi::CString,
    fs,
    os::unix::{ffi::OsStrExt as _, fs::MetadataExt, io::AsRawFd as _},
    path::{Path, PathBuf},
    str,
    time::Duration,
};
use warp_core::safe_error;

use anyhow::{anyhow, bail, ensure, Context, Result};
use channel_versions::VersionInfo;
use nix::unistd::{fchown, getgid};
use nix::{errno::Errno, unistd::getuid};
use warp_core::macos::get_bundle_path;
use warpui::{AppContext, ModelContext, SingletonEntity};

use crate::{
    appearance::AppearanceManager,
    autoupdate::{AutoupdateStage, AutoupdateState},
    channel::{Channel, ChannelState},
    safe_info,
};

use super::{
    github, release_assets_directory_url, DownloadProgress, DownloadReady, ProgressCallback,
};

// Relative path to the directory containing old executables from before an autoupdate.
//
// TODO(vorporeal): This and relevant code should be deleted after auto-updates have been
//      storing the old executable in the user application data directory for a couple
//      releases.
const OLD_EXECUTABLE_PATH: &str = "Contents/MacOS/old";

// Name of the old executable file that was kept around during an autoupdate.
const OLD_EXECUTABLE_FILE_NAME: &str = "old";

// Tmp file name used to check if the user has the correct permissions for autoupdate.
const PERMISSIONS_TMP_FILE_NAME: &str = "permission_test";

fn old_executable_file_path() -> PathBuf {
    warp_core::paths::state_dir().join(OLD_EXECUTABLE_FILE_NAME)
}

/// Removes the old executable dir from the app bundle. This is necessary because after an
/// autoupdate deleting the running executable causes the pty to not start for a reason we don't
/// fully understand. This allows to clean up old executables when the app is first launched.
pub(super) fn remove_old_executable() -> Result<()> {
    // TODO(vorporeal): This code should be deleted after auto-updates have been
    //      storing the old executable in the user application data directory for
    //      a couple releases.
    log::info!("Removing old executable dir...");
    let old_executable_path = PathBuf::from(get_bundle_path()?).join(OLD_EXECUTABLE_PATH);
    if let Ok(metadata) = fs::metadata(&old_executable_path) {
        if metadata.is_dir() {
            fs::remove_dir_all(old_executable_path)?;
        }
    }

    log::info!("Removing old executable file...");
    let old_executable_file_path = old_executable_file_path();
    if let Ok(metadata) = fs::metadata(&old_executable_file_path) {
        if metadata.is_file() {
            fs::remove_file(old_executable_file_path)?;
        }
    }

    Ok(())
}

pub(super) fn manually_download_version(
    channel: &Channel,
    version_info: &VersionInfo,
    ctx: &mut AppContext,
) {
    let url = update_url(*channel, version_info.version.as_str());
    ctx.open_url(&url);
}

/// If the autoupdate state is ready, asynchronously apply the update and cleanup the autoupdate artifacts.
///
/// The completion callback is invoked with `Ok(Some(version))` if an update was applied, and `Ok(None)` if there was no update.
/// If there was an update, but applying it failed, it's invoked with `Err(err)`.
pub(super) fn apply_update_async<F>(app: &mut AppContext, callback: F)
where
    F: FnOnce(
            &mut AutoupdateState,
            Result<Option<VersionInfo>>,
            &mut ModelContext<AutoupdateState>,
        ) + Send
        + 'static,
{
    AutoupdateState::handle(app).update(app, |autoupdate_state, ctx| {
        match autoupdate_state.stage.clone() {
            AutoupdateStage::UpdateReady {
                new_version,
                update_id,
            }
            | AutoupdateStage::Updating {
                new_version,
                update_id,
            } => {
                let update_id_clone = update_id.clone();
                // Apply the update in a background thread.
                ctx.spawn(
                    async move {
                        let result =
                            apply_update(ChannelState::channel(), &new_version, &update_id)
                                .await
                                .map(|_| Some(new_version));
                        cleanup(&update_id).await;
                        result
                    },
                    move |autoupdate_state, result, ctx| {
                        if result.is_ok() {
                            // Reset app icon to previously selected app icon
                            AppearanceManager::as_ref(ctx).set_app_icon(ctx);
                        }
                        autoupdate_state.clear_downloaded_update(&update_id_clone, ctx);
                        callback(autoupdate_state, result, ctx);
                    },
                );
            }
            _ => {
                callback(autoupdate_state, Ok(None), ctx);
            }
        }
    })
}

pub(super) fn relaunch() -> Result<()> {
    let channel = ChannelState::channel();

    // openWarp(Channel::Oss):没有代码签名,无法用 RENAME_SWAP 在原地替换 bundle。
    // 改成调 `/usr/bin/open <dmg>`,让 Finder 弹出标准挂载窗口,用户拖到
    // Applications 目录完成安装。这里不调用 `open -n bundle` 重启自己,因为
    // 当前进程在 apply_update 阶段已请求 terminate,UI 已经知道要等用户手动
    // 关闭+重开。dmg 同样在当前进程退出后再启动 Finder。
    if matches!(channel, Channel::Oss) {
        return oss_open_installer();
    }

    let bundle_path = PathBuf::from(get_bundle_path()?);

    // 启动新版 Zap 前先等待当前进程退出，避免 Dock 中短暂出现多个图标。
    // 这里用一个中间 shell 进程轮询当前 PID，进程退出后再启动新版应用。
    //
    // 每 200ms 检查一次当前进程是否仍在运行；进程退出后启动新版。
    //
    // shell 命令需要谨慎拼接：`pid` 来自当前进程且是数字，bundle 路径和
    // 环境变量值必须 shell 转义，避免路径中的元字符造成注入。
    let pid = std::process::id();
    let quoted_bundle = shell_escape::escape(bundle_path.to_string_lossy());

    let mut open_args = format!(
        "/usr/bin/open -n {} --args {}",
        quoted_bundle,
        warp_cli::finish_update_flag(),
    );
    // 测试本地通道版本 JSON 时，让新启动的二进制继续引用同一个文件，
    // 以便验证自动更新后的 changelog 展示。
    if let Ok(path) = env::var("WARP_CHANNEL_VERSIONS_PATH") {
        let quoted_path = shell_escape::escape(path.into());
        open_args.push_str(&format!(" --env WARP_CHANNEL_VERSIONS_PATH={quoted_path}"));
    }

    let relaunch_script =
        format!("while ps -p {pid} >/dev/null 2>&1; do sleep 0.2; done; {open_args}");

    log::info!("Executing relaunch command {relaunch_script:?}");
    blocking::Command::new("sh")
        .arg("-c")
        .arg(relaunch_script)
        .spawn()?;
    Ok(())
}

/// OSS macOS 安装入口:扫描 `cache_dir/autoupdate/<id>/` 找到刚下载的 dmg,
/// 等当前进程退出后用 `/usr/bin/open <dmg>` 触发 Finder 标准挂载。
fn oss_open_installer() -> Result<()> {
    // 进入这条路径前 AutoupdateState.stage 必然是 UpdateReady / Updating,
    // downloaded_update.update_id 必然存在;但我们不在 stateless 函数里访问
    // AutoupdateState,改成扫描磁盘:遍历 cache_dir/autoupdate/ 找最新 dmg。
    let mut autoupdate_dir = warp_core::paths::cache_dir();
    autoupdate_dir.push("autoupdate");

    let dmg = find_latest_dmg(&autoupdate_dir).ok_or_else(|| {
        anyhow!("openWarp: 找不到已下载的 dmg(目录: {autoupdate_dir:?})")
    })?;

    log::info!("openWarp: 准备打开安装 dmg {dmg:?}");

    let pid = std::process::id();
    let quoted_dmg = shell_escape::escape(dmg.to_string_lossy());
    // 等当前进程退出后再 open dmg。`open` 默认非阻塞,Finder 拿到 dmg 后会
    // 自动 mount 并显示挂载窗口;用户在 Finder 里拖拽到 Applications 完成升级。
    let script = format!(
        "while ps -p {pid} >/dev/null 2>&1; do sleep 0.2; done; /usr/bin/open {quoted_dmg}"
    );
    log::info!("Executing OSS install command {script:?}");
    blocking::Command::new("sh").arg("-c").arg(script).spawn()?;
    Ok(())
}

/// 在 `autoupdate/` 目录下找出最新一次下载的 dmg。OSS 只下载 dmg 不下载其他文件,
/// 按文件 mtime 取最新即可。返回 None 表示当前没有可用 dmg(异常情况)。
fn find_latest_dmg(autoupdate_dir: &Path) -> Option<PathBuf> {
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let read_dir = fs::read_dir(autoupdate_dir).ok()?;
    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Ok(inner) = fs::read_dir(&path) else {
            continue;
        };
        for inner_entry in inner.flatten() {
            let inner_path = inner_entry.path();
            if inner_path
                .extension()
                .and_then(|e| e.to_str())
                .is_none_or(|e| !e.eq_ignore_ascii_case("dmg"))
            {
                continue;
            }
            let Ok(meta) = fs::metadata(&inner_path) else {
                continue;
            };
            let Ok(mtime) = meta.modified() else {
                continue;
            };
            if newest.as_ref().is_none_or(|(_, t)| mtime > *t) {
                newest = Some((inner_path, mtime));
            }
        }
    }
    newest.map(|(p, _)| p)
}

pub async fn cleanup(update_id: &str) {
    let download_dir = get_download_dir(update_id);
    if download_dir.exists() {
        log::info!("Cleaning up download dir {:?}", &download_dir);
        if let Err(e) = async_fs::remove_dir_all(&download_dir).await {
            safe_error!(
                safe: ("Error cleaning up download dir: {e:?}"),
                full: ("Error cleaning up download dir {:?}: {:?}", &download_dir, e)
            );
        }
    }
}

/// Clean up all autoupdate directories except the specified one.
/// This helps prevent accumulation of old update directories from failed downloads,
/// race conditions, or incomplete cleanups.
pub async fn cleanup_all_except(preserve_update_id: Option<&str>) {
    let mut autoupdate_dir = warp_core::paths::cache_dir();
    autoupdate_dir.push("autoupdate");

    if !autoupdate_dir.exists() {
        return;
    }

    log::debug!("Cleaning up all autoupdate directories except {preserve_update_id:?}");

    let mut entries = match async_fs::read_dir(&autoupdate_dir).await {
        Ok(entries) => entries,
        Err(e) => {
            log::warn!("Could not read autoupdate directory {autoupdate_dir:?}: {e:?}");
            return;
        }
    };

    while let Some(entry) = entries.next().await {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                log::warn!("Error reading autoupdate directory entry: {e:?}");
                continue;
            }
        };

        let path = entry.path();
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };

        // Skip the directory we want to preserve
        if let Some(preserve_id) = preserve_update_id {
            if file_name == preserve_id {
                log::debug!("Preserving autoupdate directory: {path:?}");
                continue;
            }
        }

        let metadata = match async_fs::metadata(&path).await {
            Ok(metadata) => metadata,
            Err(e) => {
                log::warn!("Could not get metadata for {path:?}: {e:?}");
                continue;
            }
        };

        if metadata.is_dir() {
            log::debug!("Removing old autoupdate directory: {path:?}");
            if let Err(e) = async_fs::remove_dir_all(&path).await {
                log::warn!("Failed to remove autoupdate directory {path:?}: {e:?}");
            }
        }
    }
}

/// Determines if the user needs authorization in order to update Zap.
async fn needs_authorization(bundle_path: &Path) -> Result<bool> {
    // For the bundle path itself, check permissions without creating a test file so as to not
    // interfere with code signing.
    let bundle_dir_writable = permissions::is_writable(bundle_path)?;
    if !bundle_dir_writable {
        log::info!("App location is not writable, needs authorization");
        return Ok(true);
    } else {
        log::info!("App location is writable");
    }

    if let Some(bundle_parent_path) = bundle_path.parent() {
        if !is_directory_writable(bundle_parent_path).await? {
            log::info!("App parent location is not writable, needs authorization");
            return Ok(true);
        } else {
            log::info!("App parent location is writable");
        }
    }

    Ok(false)
}

/// Determines if a directory is writable as part of an update. This means:
/// * Zap can create files in the directory
/// * Zap can modify the permissions of created files
async fn is_directory_writable(directory: &Path) -> Result<bool> {
    // Just because we have writability access does not mean we can set the correct owner/group.
    // Test if we can set the owner/group on a temporarily created file. If we can, then we can
    // probably perform an update without authorization.
    let tmp_file_name = directory.join(PERMISSIONS_TMP_FILE_NAME);

    safe_info!(
        safe: ("Writing to a tmp file to determine if permissions are correct"),
        full: ("Writing to a tmp file to determine if permissions are correct in {}", directory.display())
    );

    let needs_authorization = match async_fs::File::create(&tmp_file_name).await {
        Ok(file) => {
            let fchown_result = fchown(file.as_raw_fd(), Some(getuid()), Some(getgid()));
            if let Err(err) = &fchown_result {
                log::warn!("Could not set permissions on tmp file: {err:#}");
            }

            // Only remove the tmp file if it was created - otherwise, we'll mask permission
            // errors.
            async_fs::remove_file(&tmp_file_name).await?;
            fchown_result.is_ok()
        }
        Err(e) => {
            // Obvious indicator we may need authorization.
            log::warn!("Could not create tmp file: {e:#}");
            false
        }
    };

    Ok(needs_authorization)
}

/// Verifies that the staged bundle path has a valid macOS code signature, and that its
/// team identifier matches Zap's team identifier.
async fn verify_code_signature(component: &str, path: &Path) -> Result<()> {
    // Verify the signature of the staged update bundle with team identifier
    let codesign_verify_output = Command::new("/usr/bin/codesign")
        .arg("-v")
        .arg(format!(
            "-R=certificate leaf[subject.OU] = \"{}\"",
            warp_core::macos::APPLE_TEAM_ID
        ))
        .arg(path)
        .output()
        .await?;
    ensure!(
        codesign_verify_output.status.success(),
        "Failed to verify code signature for {component} with team identifier: {codesign_verify_output:?}"
    );

    safe_info!(
        safe: ("Code signature is valid for {component}"),
        full: ("Code signature is valid for {}", path.display())
    );

    Ok(())
}

pub(super) async fn download_update_and_cleanup(
    version_info: &VersionInfo,
    update_id: &str,
    last_successful_update_id: Option<&str>,
    client: &http_client::Client,
    on_progress: ProgressCallback,
) -> Result<DownloadReady> {
    let channel = ChannelState::channel();

    // openWarp(Channel::Oss):没有 Apple Developer ID 签名,不能走官方
    // download_and_extract_binary(mount + cp + codesign verify + RENAME_SWAP)。
    // OSS 路径只把 dmg 流式下载到 cache_dir/autoupdate/<id>/,apply 时由
    // `relaunch()` 走 `open <dmg>` 让 Finder 弹标准挂载窗口,用户拖到 Applications。
    let result = if matches!(channel, Channel::Oss) {
        oss_download_dmg(channel, version_info, update_id, client, on_progress).await
    } else {
        download_and_extract_binary(channel, version_info, update_id, client, on_progress).await
    };
    if result.is_err() {
        cleanup_all_except(last_successful_update_id).await;
    }
    result
}

/// OSS 专用下载:只把 dmg 流式落盘到 `cache_dir/autoupdate/<update_id>/<dmg>`,
/// 不做挂载也不做代码签名校验。返回 `DownloadReady::Yes` 表示安装包已就绪,
/// 上层会切到 `UpdateReady`,等待用户点击"立即安装"触发 `relaunch()`。
async fn oss_download_dmg(
    channel: Channel,
    version_info: &VersionInfo,
    update_id: &str,
    client: &http_client::Client,
    on_progress: ProgressCallback,
) -> Result<DownloadReady> {
    log::info!(
        "openWarp: 下载更新 dmg, version {} on channel {channel}",
        &version_info.version
    );

    let download_dir = get_download_dir(update_id);
    async_fs::create_dir_all(&download_dir).await?;

    let dmg_path_buf = download_dmg(&channel, version_info, update_id, client, on_progress).await?;

    // 故意不做 hdiutil mount / verify_code_signature:OSS 没有 Apple
    // codesign 也不需要把 .app 拷进当前 bundle。dmg 本身就是用户要"打开"的物件。
    // 但校验 GitHub Release 元数据里的 SHA-256,防御 CDN 中间人/资产损坏。
    let asset_name = dmg_name(channel);
    if let Err(e) = super::verify_oss_asset_sha256(&dmg_path_buf, &asset_name) {
        // 校验失败时立即删除已下载文件,避免用户点击"安装"后打开损坏的 dmg。
        let _ = async_fs::remove_file(&dmg_path_buf).await;
        return Err(e);
    }
    Ok(DownloadReady::Yes)
}

/// Apply the downloaded update.
///
/// This is async and should be run in a background task.
async fn apply_update(channel: Channel, version_info: &VersionInfo, update_id: &str) -> Result<()> {
    let update_start = Instant::now();

    let bundle_path = PathBuf::from(get_bundle_path()?);
    let bundle_parent_path = bundle_path
        .parent()
        .ok_or_else(|| anyhow!("Could not get parent directory of application bundle"))?;

    // Double-check that we have permissions to apply the update.
    if !permissions::is_writable(&bundle_path)? {
        bail!("App location is not writable, cannot apply update");
    }
    if !is_directory_writable(bundle_parent_path).await? {
        bail!("App parent location is not writable, cannot apply update");
    }

    // Read a file out of the old bundle to ensure that we've triggered macOS' directory
    // permissions checks.
    let old_info_plist = bundle_path.join("Contents/Info.plist");
    if async_fs::File::open(&old_info_plist).await.is_err() {
        bail!("App location is not readable, cannot apply update");
    }

    let dmg_path = dmg_path(&channel, version_info, update_id);
    let temp_app_path = temporary_target_path(channel, version_info, &dmg_path)?;

    let staged_bundle =
        StagedBundle::for_bundle_path(channel, version_info, temp_app_path, &bundle_path).await?;

    // Copy permissions to new app
    let bundle_metadata = async_fs::metadata(&bundle_path).await?;
    async_fs::set_permissions(&staged_bundle.path, bundle_metadata.permissions()).await?;

    // Verify that the new version actually exists before proceeding
    let executable_path_buf = staged_bundle.path.join(executable_path(channel));
    if !executable_path_buf.exists() {
        bail!(
            "New executable does not exist at path: {:?}",
            executable_path_buf
        );
    }

    // Atomically rename the new app to have the same name as the old one.
    log::info!("Renaming new app to original app name");
    let from = CString::new(staged_bundle.path.as_os_str().as_bytes())?;
    let to = CString::new(bundle_path.as_os_str().as_bytes())?;

    Errno::result(unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_SWAP) })
        .context("Error swapping old and new app bundles")?;

    // Move the current running executable into a temporary directory so we can delete the
    // rest of the old bundle without removing the running executable (since removing it
    // causes the `fork` syscall to fail).
    let executable_temp_file = old_executable_file_path();
    if async_fs::metadata(executable_temp_file.as_path())
        .await
        .is_ok()
    {
        // If we performed this process already but didn't relaunch Zap, the old executable will
        // still be located in the user application data directory.  In that case, leave it there.
        log::info!("Already autoupdated without relaunching; ignoring executable from old bundle");
    } else {
        // Compute the location of the old executable (which, after the swap of the app contents,
        // is located in the "new app" directory).
        let new_app_executable_path = staged_bundle.path.join(executable_path(channel));

        log::info!(
            "Moving old executable at path {new_app_executable_path:?} into user application data dir at path {executable_temp_file:?}"
        );
        let mv_output = Command::new("mv")
            .arg(new_app_executable_path)
            .arg(executable_temp_file)
            .output()
            .await?;

        ensure!(
            mv_output.status.success(),
            "Failed to move old executable: {mv_output:?}"
        );
    }

    log::info!("Setting installed version to {:?}", &version_info);
    log::info!("Applied update in {:?}", update_start.elapsed());

    Ok(())
}

/// The staged app bundle that we're about to install. It's copied out of the `.dmg` file into a
/// temporary location.
struct StagedBundle {
    /// Path to the on-disk temporary bundle.
    path: PathBuf,
    /// Whether or not the temporary bundle was copied into the same directory as the existing app.
    /// This is only necessary if `$TMPDIR` and the app are on different filesystems.
    in_app_directory: bool,
}

impl StagedBundle {
    async fn for_bundle_path(
        channel: Channel,
        version_info: &VersionInfo,
        temp_app_path: PathBuf,
        bundle_path: &Path,
    ) -> Result<Self> {
        let temp_device_id = async_fs::metadata(&temp_app_path)
            .await
            .context("Could not get metadata for temporary app bundle")?
            .dev();
        let bundle_device_id = async_fs::metadata(bundle_path)
            .await
            .context("Could not get metadata for app bundle")?
            .dev();

        if temp_device_id == bundle_device_id {
            // The old and new app bundles are on the same filesystem (this is the expected case).
            Ok(Self {
                path: temp_app_path,
                in_app_directory: false,
            })
        } else {
            let bundle_parent_path = bundle_path
                .parent()
                .ok_or_else(|| anyhow!("Could not get parent directory of application bundle"))?;
            log::info!("Copying app contents from {temp_app_path:?} to {bundle_parent_path:?}");

            let cp_output = Command::new("cp")
                // Recursively copy the directory, preserving symlinks.
                .arg("-R")
                // Overwrite files at the destination.
                .arg("-f")
                .arg(&temp_app_path)
                .arg(bundle_parent_path)
                .output()
                .await?;

            ensure!(
                cp_output.status.success(),
                "Failed to copy app contents from temporary directory into bundle directory: {cp_output:?}"
            );

            Ok(Self {
                path: bundle_parent_path.join(versioned_app_name(channel, &version_info.version)),
                in_app_directory: true,
            })
        }
    }
}

impl Drop for StagedBundle {
    fn drop(&mut self) {
        // Clean up in the destructor so that it happens even if the installation errors.
        // If we used the original temporary app bundle, it'll get removed by the final cleanup
        // step, along with the dmg.
        if self.in_app_directory {
            log::info!("Removing temporary app bundle");
            if let Err(err) = fs::remove_dir_all(&self.path) {
                log::error!("Failed to remove temporary bundle: {err:#}");
            }
        }
    }
}

async fn download_and_extract_binary(
    channel: Channel,
    version_info: &VersionInfo,
    update_id: &str,
    client: &http_client::Client,
    on_progress: ProgressCallback,
) -> Result<DownloadReady> {
    let bundle_path = PathBuf::from(get_bundle_path()?);
    let needs_authorization = needs_authorization(bundle_path.as_path())
        .await
        .unwrap_or(true);
    if needs_authorization {
        return Ok(DownloadReady::NeedsAuthorization);
    }

    log::info!(
        "Downloading update, version {} on channel {channel}",
        &version_info.version,
    );

    let download_dir = get_download_dir(update_id);
    log::info!("Creating download dir {:?}", &download_dir);
    async_fs::create_dir_all(&download_dir).await?;

    let dmg_path = download_dmg(&channel, version_info, update_id, client, on_progress).await?;

    // Mount the downloaded dmg so we can copy out the binary.
    let mountpoint = mount_dmg(&dmg_path, update_id).await?;

    let target = temporary_target_path(channel, version_info, &dmg_path)?;
    // Copy the binary into the temporary directory where we downloaded the dmg.
    copy_app_from_dmg(&channel, &mountpoint, &target).await?;

    // Unmount the dmg once we no longer need it. This prevents lingering images from unapplied
    // updates.
    if let Err(err) = unmount_dmg(mountpoint).await {
        let err = err.context("Error unmounting dmg for update");
        crate::report_error!(&err);
    }

    // Ensure that the new app we just downloaded has both integrity (e.g. no corrupted files)
    // and validity (it was signed by us).
    // Store the executable path in a variable to prevent temporary value issues.
    let executable_path_buf = target.join(executable_path(channel));
    let verification_start = Instant::now();
    future::try_zip(
        verify_code_signature("bundle", &target),
        verify_code_signature("executable", executable_path_buf.as_path()),
    )
    .await?;

    log::info!(
        "Verified new app code signature in {:?}",
        verification_start.elapsed()
    );

    Ok(DownloadReady::Yes)
}

async fn unmount_dmg(mountpoint: PathBuf) -> Result<()> {
    let mut hdiutil_cmd = Command::new("/usr/bin/hdiutil");
    hdiutil_cmd.arg("detach");
    hdiutil_cmd.arg(&mountpoint);
    hdiutil_cmd.arg("-force");

    log::info!("Attempting to detach dmg with command \"{hdiutil_cmd:?}\"");

    let output = hdiutil_cmd.output().await?;

    ensure!(output.status.success(), "Failed to detach dmg: {output:?}");
    log::info!("hdiutil detach succeeded: {output:?}");
    Ok(())
}

async fn copy_app_from_dmg(channel: &Channel, mountpoint: &Path, target: &Path) -> Result<()> {
    let mounted_app_path = mountpoint.join(app_name(*channel));

    log::info!("Copying dmg contents from {mounted_app_path:?} to {target:?}");

    let cp_output = Command::new("cp")
        // Recursively copy the directory, preserving symlinks.
        .arg("-R")
        .arg(mounted_app_path)
        .arg(target)
        .output()
        .await?;

    ensure!(
        cp_output.status.success(),
        "Failed to copy app out of mounted dmg: {cp_output:?}"
    );

    Ok(())
}

// 10 minutes
const DMG_TIMEOUT_S: u64 = 600;

/// The temporary path for downloading the new dmg into.
fn dmg_path(channel: &Channel, version_info: &VersionInfo, update_id: &str) -> PathBuf {
    let mut dir = get_download_dir(update_id);
    let file_name = format!(
        "{}.{}.dmg",
        &version_info.version,
        app_name_prefix(*channel)
    );
    dir.push(file_name);
    dir
}

/// The temporary path for placing our downloaded app binary.
fn temporary_target_path(
    channel: Channel,
    version_info: &VersionInfo,
    dmg_path: &Path,
) -> Result<PathBuf> {
    Ok(dmg_path
        .parent()
        .ok_or_else(|| anyhow!("Could not get parent directory of downloaded DMG"))?
        .join(versioned_app_name(channel, &version_info.version)))
}

async fn download_dmg(
    channel: &Channel,
    version_info: &VersionInfo,
    update_id: &str,
    client: &http_client::Client,
    on_progress: ProgressCallback,
) -> Result<PathBuf> {
    let update_url = update_url(*channel, &version_info.version);
    log::info!("Fetching new dmg at {update_url}");
    let res = client
        .get(&update_url)
        .timeout(Duration::from_secs(DMG_TIMEOUT_S))
        .send()
        .await?
        .error_for_status()?;
    // http_client::Response 没有 content_length(),只能从 headers 拿。
    let total = res
        .headers()
        .get(http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok());
    let dmg_file = dmg_path(channel, version_info, update_id);

    // 上报 0/total 让 UI 立刻渲染进度条;后续每写一个 chunk 再 throttle 上报。
    on_progress(DownloadProgress {
        downloaded: 0,
        total,
    });

    let mut file = async_fs::File::create(&dmg_file).await?;
    let mut downloaded: u64 = 0;
    // 节流:不要每个 chunk 都上报(reqwest chunk 可能很小,UI 会被狂刷重绘)。
    // 每累积 64 KiB 或时间过 250ms 才推一次;最后一次在循环外强制 flush。
    let mut last_reported = 0u64;
    let mut last_reported_at = Instant::now();
    const REPORT_BYTES_THRESHOLD: u64 = 64 * 1024;
    const REPORT_TIME_THRESHOLD: Duration = Duration::from_millis(250);

    use futures_lite::io::AsyncWriteExt as _;
    let mut stream = res.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        if downloaded - last_reported >= REPORT_BYTES_THRESHOLD
            || last_reported_at.elapsed() >= REPORT_TIME_THRESHOLD
        {
            on_progress(DownloadProgress {
                downloaded,
                total,
            });
            last_reported = downloaded;
            last_reported_at = Instant::now();
        }
    }
    on_progress(DownloadProgress {
        downloaded,
        total,
    });
    file.sync_data().await?;

    log::info!("Wrote DMG to tempfile at {:?}", &dmg_file);
    Ok(dmg_file)
}

fn get_download_dir(update_id: &str) -> PathBuf {
    let mut dir = warp_core::paths::cache_dir();
    dir.push("autoupdate");
    dir.push(update_id);
    dir
}

fn get_mountpoint(update_id: &str) -> PathBuf {
    let mut volume = PathBuf::from("/Volumes");
    volume.push(update_id);
    volume
}

async fn mount_dmg(dmg_dir: &Path, update_id: &str) -> Result<PathBuf> {
    let volume = get_mountpoint(update_id);
    let mut hdiutil_cmd = Command::new("/usr/bin/hdiutil");
    hdiutil_cmd.args(["attach", "-mountpoint"]);
    hdiutil_cmd.arg(&volume);
    // Explanation of flags:
    // -nobrowse: Do not show the Zap DMG in Finder or similar apps.
    // -noautoopen: Do not open the Zap DMG in Finder.
    // -readonly: For safety, we mount read-only since there's no need to modify the new app version.
    // -autofsck: Ensure that the DMG contents are verified. This is on by default for quarantined images, but macOS
    //    doesn't necessarily recognize our download as such.
    hdiutil_cmd.args(["-nobrowse", "-noautoopen", "-readonly", "-autofsck"]);
    hdiutil_cmd.arg(dmg_dir);

    log::info!("Attempting to mount dmg with command \"{hdiutil_cmd:?}\"");

    let output = hdiutil_cmd.output().await?;

    ensure!(output.status.success(), "Failed to mount dmg: {output:?}");

    log::info!("hdiutil mount succeeded");
    Ok(volume)
}

fn update_url(channel: Channel, version: &str) -> String {
    let asset = dmg_name(channel);
    if matches!(channel, Channel::Oss) {
        // OSS 走 GitHub Releases:优先用 fetch_latest_release 缓存里的真实
        // browser_download_url(以防仓库被 redirect / asset 改名)。缓存为空时
        // 拼一个标准 `releases/download/<tag>/<asset>` 的兜底 URL,tag 直接用
        // VersionInfo.version 加 `v` 前缀(VersionInfo 已经 trim 过 `v`)。
        if let Some(release) = github::cached_release() {
            if let Some(found) = release.find_asset(&asset) {
                return found.browser_download_url.clone();
            }
            log::warn!(
                "openWarp: cached release tag {} 没有名为 {asset} 的资产,回退到 tag URL",
                release.tag_name
            );
        }
        return format!(
            "https://github.com/zerx-lab/warp/releases/download/v{version}/{asset}"
        );
    }
    format!(
        "{}/{}",
        release_assets_directory_url(channel, version),
        asset
    )
}

fn app_name(channel: Channel) -> String {
    format!("{}.app", app_name_prefix(channel))
}

fn versioned_app_name(channel: Channel, version: &str) -> String {
    format!("{}({}).app", app_name_prefix(channel), version)
}

fn dmg_name(channel: Channel) -> String {
    // If the user is on an Apple Silicon Mac, download an arm64-only bundle.
    let is_arm64 = command::blocking::Command::new("uname")
        .arg("-m")
        .output()
        .is_ok_and(|output| output.stdout.starts_with(b"arm64"));

    // openWarp GitHub Release 资产名固定使用 `Zap-arm64.dmg` / `Zap-intel.dmg`
    // (来自 .github/workflows 的命名约定),与 `app_name_prefix("zap-oss")` 不一致。
    // 这里只对 OSS 写死,不会影响官方 channel 的 universal 命名。
    if matches!(channel, Channel::Oss) {
        return if is_arm64 {
            "Zap-arm64.dmg".to_string()
        } else {
            "Zap-intel.dmg".to_string()
        };
    }

    if is_arm64 {
        return format!("{}-arm64.dmg", app_name_prefix(channel));
    }

    // Otherwise, download a universal bundle.
    format!("{}.dmg", app_name_prefix(channel))
}

fn app_name_prefix(channel: Channel) -> &'static str {
    match channel {
        Channel::Stable => "Zap",
        Channel::Preview => "WarpPreview",
        Channel::Local => "warp",
        Channel::Integration => "integration",
        Channel::Dev => "WarpDev",
        Channel::Oss => "zap-oss",
    }
}

fn executable_name(channel: Channel) -> &'static str {
    match channel {
        Channel::Stable => "stable",
        Channel::Preview => "preview",
        Channel::Local => "warp",
        Channel::Integration => "integration",
        Channel::Dev => "dev",
        Channel::Oss => "zap-oss",
    }
}

fn executable_path(channel: Channel) -> String {
    if ChannelState::is_release_bundle() {
        format!("Contents/MacOS/{}", executable_name(channel))
    } else {
        executable_name(channel).to_owned()
    }
}
