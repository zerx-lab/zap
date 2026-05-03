use crate::server::telemetry::TelemetryEvent;
use anyhow::anyhow;
use anyhow::{bail, Context as _, Result};
use channel_versions::VersionInfo;
use command::blocking::Command;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;
use std::{fs, io};
use std::{io::Write as _, time::Duration};
use tempfile::TempPath;
use warp_core::channel::{Channel, ChannelState};
use warpui::AppContext;

use super::{github, release_assets_directory_url, DownloadReady};
use crate::util::windows::install_dir;

lazy_static! {
    /// The path to the temporary file that stores the installer for the new update.
    static ref INSTALLER_PATH: Arc<Mutex<Option<TempPath>>> = Default::default();
}

/// Download the Inno Setup install wizard, the same one users run on the first Warp install, and
/// place it into the "data dir".
pub(super) async fn download_update_and_cleanup(
    version_info: &VersionInfo,
    _update_id: &str,
    client: &http_client::Client,
) -> Result<DownloadReady> {
    const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);

    // openWarp 走 GitHub Release:下载到用户 Downloads 目录,完成后用 explorer
    // 打开目录并高亮 installer。不走 Inno Setup 静默安装,由用户手动运行。
    if matches!(ChannelState::channel(), Channel::Oss) {
        return download_oss_to_downloads(client).await;
    }

    let installer_file_name = installer_file_name()?;
    let url = format!(
        "{}/{}",
        release_assets_directory_url(ChannelState::channel(), &version_info.version),
        installer_file_name
    );

    // Create a temporary file that we'll write the download into.
    let mut already_exists = false;
    let mut new_installer = tempfile::Builder::new()
        .rand_bytes(0)
        .suffix(&format!("{}-{}", version_info.version, installer_file_name))
        .make(|path| {
            already_exists = path.is_file();
            if already_exists {
                File::open(path)
            } else {
                File::create(path)
            }
        })?;

    if !already_exists {
        log::info!("Downloading {url} to {}...", new_installer.path().display());

        let response = client
            .get(&url)
            .timeout(DOWNLOAD_TIMEOUT)
            .send()
            .await?
            .error_for_status()?;
        new_installer
            .as_file_mut()
            .write_all(&response.bytes().await?)?;
    }

    *INSTALLER_PATH.lock() = Some(new_installer.into_temp_path());

    Ok(DownloadReady::Yes)
}

const UPDATE_LOG_FILENAME: &str = "warp_update.log";

fn autoupdate_log_file() -> Result<PathBuf> {
    warp_logging::log_directory().map(|dir| dir.join(UPDATE_LOG_FILENAME))
}

/// Checks the autoupdate log file from a previous update attempt.
/// Sends telemetry for specific known issues, and sends a Sentry event if errors are found.
/// The log file is renamed after processing to avoid duplicate reports on subsequent launches.
pub(super) fn check_and_report_update_errors(ctx: &mut AppContext) {
    let log_path = match autoupdate_log_file() {
        Ok(path) => path,
        Err(e) => {
            log::warn!("Failed to determine autoupdate log file path: {e:#}");
            return;
        }
    };

    // Inno Setup logs use the system's active codepage (often Windows-1252), not UTF-8.
    // We read as raw bytes to avoid silently skipping non-UTF-8 log files.
    let contents = match fs::read(&log_path) {
        Ok(contents) => contents,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            log::info!("No autoupdate logs found");
            return;
        }
        Err(e) => {
            log::warn!("Failed to read autoupdate log file: {e:#}");
            return;
        }
    };

    let contents_lowercase = contents.to_ascii_lowercase();

    let has_unable_to_close = memchr::memmem::find(
        &contents_lowercase,
        b"setup was unable to automatically close all applications",
    )
    .is_some();
    if has_unable_to_close {
        crate::send_telemetry_sync_from_app_ctx!(
            TelemetryEvent::AutoupdateUnableToCloseApplications,
            ctx
        );
    }

    let has_file_in_use = memchr::memmem::find(
        &contents_lowercase,
        b"the process cannot access the file because it is being used by another process",
    )
    .is_some();
    if has_file_in_use {
        crate::send_telemetry_sync_from_app_ctx!(TelemetryEvent::AutoupdateFileInUse, ctx);
    }

    // Fired when the mutex polling loop timed out and a force-kill was attempted.
    let has_mutex_timeout =
        memchr::memmem::find(&contents_lowercase, b"warp mutex still held after timeout").is_some();
    if has_mutex_timeout {
        crate::send_telemetry_sync_from_app_ctx!(TelemetryEvent::AutoupdateMutexTimeout, ctx);
    }

    // Fired when taskkill returned non-zero after the mutex timeout.
    let has_forcekill_failed =
        memchr::memmem::find(&contents_lowercase, b"force-kill failed for").is_some();
    if has_forcekill_failed {
        crate::send_telemetry_sync_from_app_ctx!(TelemetryEvent::AutoupdateForcekillFailed, ctx);
    }

    // openWarp 闭源遥测剥离 P2:原 #[cfg(feature = "crash_reporting")] 块会把 autoupdate 失败
    // 日志(完整文件内容作为 sentry attachment)上报到 Warp 官方 Sentry。剥离后改为本地
    // log 计数提示——日志文件已落本地(被下方 .log.reported 重命名保留),用户/调试需要时
    // 直接看本地文件。`contents_lowercase` 仅用于 sentry 上报路径判断,一并去除。
    #[cfg(feature = "crash_reporting")]
    {
        const IGNOREABLE_ERRORS: &[&[u8]] = &[
            b"there is not enough space on the disk",
            b"setprocessmitigationpolicy failed with error code 87",
        ];

        let mut error_count = memchr::memmem::find_iter(&contents_lowercase, b"error").count();
        for pattern in IGNOREABLE_ERRORS {
            let ignoreable_count = memchr::memmem::find_iter(&contents_lowercase, pattern).count();
            error_count = error_count.saturating_sub(ignoreable_count);
        }

        if error_count > 0 {
            log::error!(
                "openWarp: Windows auto-update log contains {error_count} error(s) (log: {:?})",
                log_path
            );
        }
        let _ = &contents;
    }

    // Rename the log file to avoid duplicate reports on subsequent launches.
    // We keep the file around so the user can still view it or attach it to a GitHub issue.
    let reported_path = log_path.with_extension("log.reported");
    if let Err(e) = fs::rename(&log_path, &reported_path) {
        log::warn!("Failed to rename autoupdate log file after reporting: {e:#}");
    }
}

pub(super) fn relaunch() -> Result<()> {
    // openWarp 仅下载 installer 到 Downloads,由用户手动运行,不在此处拉起 Inno Setup。
    if matches!(ChannelState::channel(), Channel::Oss) {
        log::info!("openWarp: 跳过 Inno Setup 自动安装,installer 已落 Downloads。");
        return Ok(());
    }

    let install_dir = install_dir()?;
    let Some(installer_path) = INSTALLER_PATH.lock().take() else {
        bail!("No installer path");
    };

    let log_arg = match autoupdate_log_file() {
        Ok(dir) => format!("/LOG={}", dir.display()),
        Err(e) => {
            log::warn!("Failed to determine location for autoupdate logs: {e:#}");
            "/LOG".to_string()
        }
    };

    // The Inno Setup install wizard will run without user input. It will re-launch Warp after
    // installing the update files.
    // https://jrsoftware.org/ishelp/index.php?topic=setupcmdline
    Command::new(&installer_path)
        .args([
            // Skip asking the user to confirm.
            "/SP-",
            // Do not prompt the user for anything. Note that we do not use "VERYSILENT" so that a
            // progress bar is still shown. This is useful since the update process may take a few
            // seconds.
            "/SILENT",
            // Do not provide a cancel button on the progress bar page.
            "/NOCANCEL",
            // Indicate that restarting Windows is not necessary.
            "/NORESTART",
            &log_arg,
            "/update=1",
            // Do not forcibly kill Warp via RestartManager. The installer will wait for
            // Warp to exit naturally by polling the single-instance mutex instead.
            "/NOCLOSEAPPLICATIONS",
            &format!("/DIR={}", install_dir.display()),
        ])
        .spawn()?;

    // DEV ONLY: Sleep after spawning the installer so this process is still alive
    // when Inno Setup tries to overwrite files. This reliably reproduces the
    // auto-update race condition (APP-3702) for testing.
    if matches!(ChannelState::channel(), Channel::Dev) {
        log::info!("DEV: Sleeping 10s after spawning installer to reproduce update race");
        std::thread::sleep(Duration::from_secs(10));
    }

    Ok(())
}

fn installer_file_name() -> Result<String> {
    let app_name_prefix = app_name_prefix(ChannelState::channel());

    // For example, on arm64 this is WarpSetup-arm64.exe and on x64 this is
    // WarpSetup.exe.
    if cfg!(target_arch = "aarch64") {
        Ok(format!("{app_name_prefix}Setup-arm64.exe"))
    } else if cfg!(target_arch = "x86_64") {
        Ok(format!("{app_name_prefix}Setup.exe"))
    } else {
        Err(anyhow!(
            "Could not construct setup file name for unsupported architecture"
        ))
    }
}

fn app_name_prefix(channel: Channel) -> &'static str {
    match channel {
        Channel::Stable => "Warp",
        Channel::Preview => "WarpPreview",
        Channel::Local => "warp",
        Channel::Integration => "integration",
        Channel::Dev => "WarpDev",
        // 与 script/windows/bundle.ps1 OSS 分支 INSTALLER_NAME=OpenWarp+Setup 对齐,
        // 这样 GitHub Release 资产名 OpenWarpSetup.exe 能被 installer_file_name() 正确生成。
        Channel::Oss => "OpenWarp",
    }
}

/// openWarp(Channel::Oss)专用下载路径:从 GitHub Release 拉 installer 落到 Downloads,
/// 完成后用 explorer 打开目录并高亮文件,不走 Inno Setup 自动安装。
async fn download_oss_to_downloads(client: &http_client::Client) -> Result<DownloadReady> {
    const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);

    let installer_name = installer_file_name()?;

    let release = match github::cached_release() {
        Some(r) => r,
        None => github::fetch_latest_release(client).await?,
    };

    let asset = release.find_asset(&installer_name).with_context(|| {
        format!(
            "GitHub Release {} 缺少资产 {installer_name},请前往 {} 手动下载",
            release.tag_name, release.html_url
        )
    })?;

    let download_dir = dirs::download_dir()
        .ok_or_else(|| anyhow!("无法定位用户下载目录(dirs::download_dir 返回 None)"))?;
    if !download_dir.exists() {
        fs::create_dir_all(&download_dir).with_context(|| {
            format!("创建下载目录失败: {}", download_dir.display())
        })?;
    }
    let target_path = download_dir.join(&installer_name);

    // 已存在且大小一致 → 跳过下载(GitHub asset.size 是权威值)
    let already_downloaded = match fs::metadata(&target_path) {
        Ok(meta) => meta.len() == asset.size,
        Err(_) => false,
    };

    if already_downloaded {
        log::info!(
            "openWarp installer 已存在,跳过下载: {} ({} bytes)",
            target_path.display(),
            asset.size
        );
    } else {
        log::info!(
            "Downloading {} to {} ...",
            asset.browser_download_url,
            target_path.display()
        );
        let bytes = client
            .get(asset.browser_download_url.as_str())
            .timeout(DOWNLOAD_TIMEOUT)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        let mut file = File::create(&target_path)
            .with_context(|| format!("创建文件失败: {}", target_path.display()))?;
        file.write_all(&bytes)?;
        log::info!("openWarp installer 下载完成: {}", target_path.display());
    }

    // 用 explorer /select,<file> 打开下载目录并高亮文件;失败仅记日志,不阻塞下载结果。
    if let Err(e) = Command::new("explorer")
        .arg(format!("/select,{}", target_path.display()))
        .spawn()
    {
        log::warn!("打开 explorer 失败(已下载完成): {e:#}");
    }

    Ok(DownloadReady::Yes)
}
