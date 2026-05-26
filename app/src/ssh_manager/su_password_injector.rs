//! su 密码确认提示。持续监听 PTY 输出,当检测到用户输入 `su root` / `su - root`
//! 等切换到 root 的命令后出现密码提示时,弹出确认菜单,用户确认后注入 root 密码。
//!
//! 仅为 root 目标注入,`su lg` 等切换到其他用户不触发。
//! 先等待 shell prompt 出现(表示 SSH 登录已完成)再开始检测,避免与登录密码冲突。
//! 使用 `spawn_stream_local` + `stream!` 实现持续监听,每次 `su root` 都会触发。

use std::sync::Arc;
use std::time::Duration;

use async_broadcast::InactiveReceiver;
use async_stream::stream;
use lazy_static::lazy_static;
use regex::bytes::Regex;
use warpui::r#async::FutureExt;
use warpui::{ViewContext, WeakViewHandle};
use zeroize::Zeroizing;

use crate::ssh_manager::shell_prompt::bytes_look_like_shell_prompt;
use crate::terminal::TerminalView;

const SLIDING_WINDOW_BYTES: usize = 8 * 1024;
const BUFFER_HARD_LIMIT: usize = 16 * 1024;
/// 阶段 1 等待 shell prompt 的最大时长。超时则放弃整个 stream(并在
/// `on_done` 里把 in_flight 复位)。
const SHELL_READY_TIMEOUT: Duration = Duration::from_secs(30);

lazy_static! {
    /// 密码提示符正则 — 严格匹配两类:
    /// 1. `password` / `passphrase` / `密码` 行尾带半角冒号 `:` 或全角冒号 `：`
    /// 2. 银河麒麟 V10 的无冒号 `输入密码`
    ///
    /// 旧实现把冒号设为可选,任何含 "password" 的行尾(如
    /// `Your password has expired`)都会假阳性。
    static ref PASSWORD_PROMPT_REGEX: Regex = Regex::new(
        r"(?im)(?:(?:password|passphrase|密码)[^\n]*(?::|：)\s*$|输入密码\s*$)"
    )
    .expect("su password prompt regex must compile");

    /// su 命令正则 — 匹配目标为 root 的 su 命令(行尾):
    /// `su` / `su -` / `su -l` / `su --login` / `su root` / `su - root` /
    /// `su -l root` / `su --login root`。不匹配 `su lg` / `su - lg` 等切到
    /// 其他用户的形式;`sudo su` 因 `\bsu` 单词边界仍能命中尾部的 `su`。
    static ref SU_ROOT_CMD_REGEX: Regex =
        Regex::new(r"(?m)\bsu(?:\s+(?:-l?|--login|-))*(?:\s+root)?\s*$")
            .expect("su root cmd regex must compile");
}

/// 在 owner 上下文 spawn su 密码持续监听 stream。
pub fn spawn_su_password_injector<O>(
    pty_reads_rx: Option<InactiveReceiver<Arc<Vec<u8>>>>,
    terminal_view: WeakViewHandle<TerminalView>,
    root_password: Zeroizing<String>,
    ctx: &mut ViewContext<O>,
) where
    O: warpui::View + 'static,
{
    let Some(rx) = pty_reads_rx else {
        log::debug!("ssh su password injector: no pty_reads_rx — skip");
        return;
    };
    if root_password.is_empty() {
        log::debug!("ssh su password injector: empty root password — skip");
        return;
    }

    // 设置 in-flight 标志,阻止 OneKey 凭据选择框在等待 shell prompt 期间弹出。
    if let Some(view) = terminal_view.upgrade(ctx) {
        view.update(ctx, |view, _| {
            view.set_ssh_secret_auto_injection_in_flight(true);
        });
    }

    let prompt_stream = stream! {
        let mut active = rx.activate_cloned();
        let mut buf: Vec<u8> = Vec::with_capacity(SLIDING_WINDOW_BYTES);

        // 阶段 1: 等待 shell prompt(SHELL_READY_TIMEOUT 超时),表示登录完成
        loop {
            match active.recv().with_timeout(SHELL_READY_TIMEOUT).await {
                Ok(Ok(chunk)) => {
                    buf.extend_from_slice(&chunk);
                    if buf.len() > BUFFER_HARD_LIMIT {
                        let drop_n = buf.len() - SLIDING_WINDOW_BYTES;
                        buf.drain(..drop_n);
                    }
                    if bytes_look_like_shell_prompt(&buf) {
                        break;
                    }
                }
                _ => return,
            }
        }

        // 阶段 2: 持续检测 su root + 密码提示,每次 yield 后继续监听
        buf.clear();
        while let Ok(chunk) = active.recv().await {
            buf.extend_from_slice(&chunk);
            if buf.len() > BUFFER_HARD_LIMIT {
                let drop_n = buf.len() - SLIDING_WINDOW_BYTES;
                buf.drain(..drop_n);
            }
            if PASSWORD_PROMPT_REGEX.is_match(&buf) && is_su_to_root(&buf) {
                buf.clear();
                yield ();
            }
        }
    };

    // on_done 必须把 in_flight 复位:阶段 1(等 shell prompt)若超时/EOF 直接
    // `return` 退出 stream,此时尚未走过 on_item,若不在 on_done 里复位,
    // OneKey 在该终端会被永久挡住。
    let terminal_view_done = terminal_view.clone();
    let _ = ctx.spawn_stream_local(
        prompt_stream,
        move |_owner, (), ctx| {
            let Some(view) = terminal_view.upgrade(ctx) else {
                return;
            };
            view.update(ctx, |view, ctx| {
                view.su_root_password = Some(root_password.clone());
                view.show_su_root_confirm_menu(ctx);
                view.set_ssh_secret_auto_injection_in_flight(false);
            });
        },
        move |_owner, ctx| {
            if let Some(view) = terminal_view_done.upgrade(ctx) {
                view.update(ctx, |view, _| {
                    view.set_ssh_secret_auto_injection_in_flight(false);
                });
            }
        },
    );
}

/// 检查缓冲区中是否包含目标为 root 的 su 命令。
fn is_su_to_root(buf: &[u8]) -> bool {
    SU_ROOT_CMD_REGEX.is_match(buf)
}

#[cfg(test)]
#[path = "su_password_injector_tests.rs"]
mod tests;
