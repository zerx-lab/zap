//! SSH 连接成功后自动执行启动命令。等待 shell prompt 出现后,
//! 将 startup_command 写入 PTY + `\n`,一次性注入后退出。

use std::sync::Arc;
use std::time::Duration;

use async_broadcast::InactiveReceiver;
use warpui::r#async::FutureExt;
use warpui::{ViewContext, WeakViewHandle};

use crate::terminal::TerminalView;

const INJECT_TIMEOUT: Duration = Duration::from_secs(30);
const SLIDING_WINDOW_BYTES: usize = 8 * 1024;
const BUFFER_HARD_LIMIT: usize = 16 * 1024;

/// 在 owner 上下文 spawn 启动命令注入任务。
pub fn spawn_startup_command_injector<O>(
    pty_reads_rx: Option<InactiveReceiver<Arc<Vec<u8>>>>,
    terminal_view: WeakViewHandle<TerminalView>,
    startup_command: String,
    ctx: &mut ViewContext<O>,
) where
    O: warpui::View + 'static,
{
    let Some(rx) = pty_reads_rx else {
        log::debug!("ssh startup command injector: no pty_reads_rx — skip");
        return;
    };
    if startup_command.is_empty() {
        log::debug!("ssh startup command injector: empty command — skip");
        return;
    }

    let future = async move {
        match wait_for_shell_prompt(rx).with_timeout(INJECT_TIMEOUT).await {
            Ok(true) => Some(startup_command),
            Ok(false) | Err(_) => None,
        }
    };
    ctx.spawn(future, move |_owner, cmd_opt, ctx| {
        let Some(view) = terminal_view.upgrade(ctx) else {
            log::debug!("ssh startup command injector: terminal view dropped");
            return;
        };
        let Some(cmd) = cmd_opt else {
            log::debug!("ssh startup command injector: no shell prompt detected within timeout");
            return;
        };
        view.update(ctx, |view, ctx| {
            let mut bytes = cmd.as_bytes().to_vec();
            bytes.push(b'\n');
            view.write_to_pty(bytes, ctx);
        });
    });
}

async fn wait_for_shell_prompt(rx: InactiveReceiver<Arc<Vec<u8>>>) -> bool {
    let mut active = rx.activate_cloned();
    let mut buf: Vec<u8> = Vec::with_capacity(SLIDING_WINDOW_BYTES);
    while let Ok(chunk) = active.recv().await {
        buf.extend_from_slice(&chunk);
        if buf.len() > BUFFER_HARD_LIMIT {
            let drop_n = buf.len() - SLIDING_WINDOW_BYTES;
            buf.drain(..drop_n);
        }
        if bytes_look_like_shell_prompt(&buf) {
            return true;
        }
    }
    false
}

fn bytes_look_like_shell_prompt(bytes: &[u8]) -> bool {
    let tail = if bytes.len() > 256 {
        &bytes[bytes.len() - 256..]
    } else {
        bytes
    };
    if tail.ends_with(b"$ ") || tail.ends_with(b"# ") || tail.ends_with(b"> ") {
        return true;
    }
    if tail.ends_with(&[0xe2, 0x9d, 0xaf, 0x20])
        || tail.ends_with(&[0xe2, 0x96, 0xb6, 0x20])
        || tail.ends_with(&[0xc2, 0xbb, 0x20])
        || tail.ends_with(&[0xce, 0xbb, 0x20])
        || tail.ends_with(&[0xe2, 0x86, 0x92, 0x20])
    {
        return true;
    }
    false
}
