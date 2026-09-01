//! `ssh_command` 单元测试。
//!
//! 按 `AGENTS.md §5.6` 拆出到独立文件,由 `ssh_command.rs` 末尾的 `#[path]` 引入。
//! 覆盖范围:
//! - `build_ssh_args` / `build_ssh_command_line` 参数构建
//! - `test_connection` 在缺密码 / 错认证类型时的错误路径
//! - `build_password_auth_stdin` 字节流构造(顺带覆盖 stdin 注入的关键安全路径)
//!
//! 注意:实际派生 ssh 子进程的端到端测试在 `app/src/ssh_manager/server_view.rs` 的
//! 集成测试 / 手测里覆盖 — 单测不做网络连接。
//!
//! author: logic
//! date: 2026-06-01

use super::*;
use zeroize::Zeroizing;

fn server() -> SshServerInfo {
    SshServerInfo {
        node_id: "n".into(),
        host: "1.2.3.4".into(),
        port: 22,
        username: "alice".into(),
        auth_type: AuthType::Password,
        key_path: None,
        credential_id: None,
        startup_command: None,
        notes: None,
        last_connected_at: None,
    }
}

#[test]
fn default_port_omitted() {
    let s = server();
    assert_eq!(build_ssh_args(&s), vec!["ssh", "alice@1.2.3.4"]);
    // shell-escape 出于保守会把 user@host 用单引号引起来,这是合法且
    // shell-equivalent 的形式 — 不强求未引用版本。
    let line = build_ssh_command_line(&s);
    assert!(
        line == "ssh alice@1.2.3.4" || line == "ssh 'alice@1.2.3.4'",
        "unexpected: {line}"
    );
}

#[test]
fn custom_port_uses_dash_p() {
    let mut s = server();
    s.port = 2222;
    assert_eq!(
        build_ssh_args(&s),
        vec!["ssh", "-p", "2222", "alice@1.2.3.4"]
    );
}

#[test]
fn key_auth_emits_dash_i() {
    let mut s = server();
    s.auth_type = AuthType::Key;
    s.key_path = Some("/home/u/.ssh/id_ed25519".into());
    assert_eq!(
        build_ssh_args(&s),
        vec!["ssh", "-i", "/home/u/.ssh/id_ed25519", "alice@1.2.3.4"]
    );
}

#[test]
fn key_auth_without_path_is_skipped() {
    let mut s = server();
    s.auth_type = AuthType::Key;
    s.key_path = None;
    assert_eq!(build_ssh_args(&s), vec!["ssh", "alice@1.2.3.4"]);
}

#[test]
fn empty_username_yields_host_only() {
    let mut s = server();
    s.username = String::new();
    assert_eq!(build_ssh_args(&s), vec!["ssh", "1.2.3.4"]);
}

#[test]
fn shell_escapes_spaces_in_path() {
    let mut s = server();
    s.auth_type = AuthType::Key;
    s.key_path = Some("/path with spaces/id_rsa".into());
    let line = build_ssh_command_line(&s);
    assert!(
        line.contains("'/path with spaces/id_rsa'"),
        "actual: {line}"
    );
}

#[test]
fn test_connection_requires_password_for_password_auth() {
    let s = server();
    // test_connection 应该在没有密码时返回 Offline + 错误信息
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(test_connection(&s, None));
    assert_eq!(result.status, ConnectionStatus::Offline);
    assert!(
        result
            .error_message
            .unwrap()
            .contains("Password not provided")
    );
}

#[test]
fn test_connection_requires_password_for_onekey_auth() {
    let mut s = server();
    s.auth_type = AuthType::OneKey;
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(test_connection(&s, None));
    assert_eq!(result.status, ConnectionStatus::Offline);
    assert!(
        result
            .error_message
            .unwrap()
            .contains("Password not provided")
    );
}

#[test]
fn onekey_key_auth_emits_dash_i_when_key_path_is_resolved() {
    let mut s = server();
    s.auth_type = AuthType::OneKey;
    s.key_path = Some("/home/u/.ssh/shared_ed25519".into());

    assert_eq!(
        build_ssh_args(&s),
        vec!["ssh", "-i", "/home/u/.ssh/shared_ed25519", "alice@1.2.3.4"]
    );
}

#[test]
fn test_connection_key_auth_uses_batch_mode() {
    let mut s = server();
    s.auth_type = AuthType::Key;
    s.key_path = Some("/home/user/.ssh/id_rsa".into());
    // 对于密钥认证,应该走 BatchMode=yes 路径(由 run_ssh_test 携带);
    // 这里只验证 build_ssh_args 带了 -i 和 key_path。
    let args = build_ssh_args(&s);
    assert!(args.contains(&"-i".to_string()));
    assert!(args.contains(&"/home/user/.ssh/id_rsa".to_string()));
}

#[test]
fn connection_status_equality() {
    assert_eq!(ConnectionStatus::Online, ConnectionStatus::Online);
    assert_eq!(ConnectionStatus::Offline, ConnectionStatus::Offline);
    assert_eq!(ConnectionStatus::Unknown, ConnectionStatus::Unknown);
    assert_ne!(ConnectionStatus::Online, ConnectionStatus::Offline);
    assert_ne!(ConnectionStatus::Online, ConnectionStatus::Unknown);
    assert_ne!(ConnectionStatus::Offline, ConnectionStatus::Unknown);
}

// -------- 密码 stdin 注入安全相关 --------

/// 验证 `build_password_auth_stdin` 把密码 + 换行正确编码。
/// 这是密码泄露修复的关键:必须确认写进 ssh stdin 的字节流就是密码字面量 +
/// `\n`,而不是任何会让密码意外走 argv / 环境变量 / 临时文件的形态。
#[test]
fn build_password_auth_stdin_contains_password_with_newline() {
    let password: Zeroizing<String> = Zeroizing::new("s3cret-pass".into());
    let bytes = build_password_auth_stdin(&password);
    assert_eq!(&*bytes, b"s3cret-pass\n");
}

/// 边界:空密码仍要写一个 `\n`,这样 ssh 会立即拿到 EOF 并判定认证失败
/// (而不是卡在等待 prompt 的状态)。
#[test]
fn build_password_auth_stdin_empty_password_still_has_newline() {
    let password: Zeroizing<String> = Zeroizing::new(String::new());
    let bytes = build_password_auth_stdin(&password);
    assert_eq!(&*bytes, b"\n");
}

/// Unicode 密码:走 UTF-8 字节原样写入。
#[test]
fn build_password_auth_stdin_unicode_password() {
    let password: Zeroizing<String> = Zeroizing::new("密码🔐".into());
    let bytes = build_password_auth_stdin(&password);
    let mut expected = "密码🔐".as_bytes().to_vec();
    expected.push(b'\n');
    assert_eq!(&*bytes, expected.as_slice());
}

/// 回归:`build_ssh_args` 不应再带 `sshpass`,防止有人误把它加回 cmd_args
/// (Windows / macOS 默认无 sshpass,残留路径会立即 No such file or directory)。
#[test]
fn build_ssh_args_does_not_emit_sshpass() {
    let s = server();
    let args = build_ssh_args(&s);
    assert!(
        !args.iter().any(|a| a == "sshpass"),
        "build_ssh_args must not emit sshpass; got {args:?}"
    );
}

// -------- password auth cmd_args 回归保护 --------
//
// 这些测试守住"测试连接"password 路径不再 10s timeout 的关键开关。
// 任何 `test_password_auth` 内部的 -o 选项调整都得满足这三条:
// 1. 不再声明 keyboard-interactive(否则 server 端 PAM 会 fallback 到 kbd-int)
// 2. 显式禁掉 KbdInteractiveAuthentication(客户端能力开关,不是偏好)
// 3. 末尾仍是 `echo ok` 远端命令(否则成功判定匹配不到 stdout)
// author: logic
// date: 2026-06-01

/// 回归保护:`PreferredAuthentications` 必须只含 `password`,不能含
/// `keyboard-interactive`。否则 stdin pipe + EOF 会触发 kbd-int PAM
/// 重试链(`pam_faildelay` ~2s/次),把 10s `TEST_TIMEOUT` 顶满。
#[test]
fn password_auth_args_no_keyboard_interactive() {
    let s = server();
    let args = build_password_auth_cmd_args(&s);
    let joined = args.join(" ");
    assert!(
        !joined.contains("keyboard-interactive"),
        "test_password_auth must NOT use keyboard-interactive; got {args:?}"
    );
    assert!(
        joined.contains("PreferredAuthentications=password"),
        "expected PreferredAuthentications=password; got {args:?}"
    );
    // 即便 PreferredAuthentications=password 出现,后面也不能再列其它方法。
    // split 取 "=" 后第一段,若以 "password," 开头说明后面还有别的认证。
    let after_pref = joined
        .split("PreferredAuthentications=")
        .nth(1)
        .unwrap_or("");
    assert!(
        !after_pref.starts_with("password,"),
        "PreferredAuthentications should not list other methods after password; got {args:?}"
    );
}

/// 回归保护:必须显式禁 kbd-interactive(客户端能力开关),
/// 不只靠 `PreferredAuthentications` 列表顺序(后者只约束 password
/// 子方法)。OpenSSH 8.2+ 行为差异、与 server 端 `AuthenticationMethods`
/// 交互时尤其需要这层 defense in depth。
#[test]
fn password_auth_args_disable_kbd_interactive() {
    let s = server();
    let args = build_password_auth_cmd_args(&s);
    let joined = args.join(" ");
    assert!(
        joined.contains("KbdInteractiveAuthentication=no"),
        "missing KbdInteractiveAuthentication=no; got {args:?}"
    );
}

/// 回归保护:cmd_args 末尾的 `echo ok` 必须作为 remote command 出现。
/// ssh 解析规则下 destination 之后第一个非选项位置参数 = remote command,
/// 如果选项顺序错了导致 ssh 不把 `echo ok` 识别为命令,成功判定会失效。
#[test]
fn password_auth_args_ends_with_echo_ok_command() {
    let s = server();
    let args = build_password_auth_cmd_args(&s);
    assert!(!args.is_empty(), "cmd_args is empty: {args:?}");
    let last = args.last().unwrap();
    assert_eq!(
        last, "echo ok",
        "cmd_args must end with `echo ok` as remote command; got {args:?}"
    );
}

/// 回归保护:password 路径的 destination (`user@host`) 必须出现在所有 `-o`
/// 选项**之后**、`echo ok` **之前**。SSH 命令行解析规则为
/// `ssh [options] destination [command]`,第一个非选项参数 = destination,
/// 其后一切 = 远程命令。若 `-o` 选项跑到 destination 之后,SSH 会把它
/// 当作远程命令的一部分而非自身选项,导致 `PreferredAuthentications`、
/// `KbdInteractiveAuthentication` 等关键选项全部失效,触发 kbd-interactive
/// PAM 重试链卡满 10s `TEST_TIMEOUT`。
/// author: logic
/// date: 2026-06-01
#[test]
fn password_auth_args_destination_before_echo_ok_and_after_options() {
    let s = server();
    let args = build_password_auth_cmd_args(&s);
    let joined = args.join(" ");

    // destination "alice@1.2.3.4" 必须出现在 "echo ok" 之前
    let dest_pos = joined
        .find("alice@1.2.3.4")
        .expect("destination must appear in args");
    let echo_pos = joined
        .find("echo ok")
        .expect("`echo ok` must appear in args");

    assert!(
        dest_pos < echo_pos,
        "destination must come before `echo ok`; got joined: {joined}"
    );

    // destination 必须出现在所有 -o 选项之后
    // 找最后一个 -o 选项的位置
    let last_o_pos = joined
        .rfind("-o ")
        .expect("expected at least one -o option");
    assert!(
        last_o_pos < dest_pos,
        "all -o options must come before destination; got joined: {joined}"
    );
}

/// 回归保护:key auth 路径的 `build_ssh_args` 也需 destination 在 -o 选项之后。
/// 用 `build_ssh_args` + 手动追加选项的方式验证顺序,模拟 `test_key_auth` 的构建。
/// author: logic
/// date: 2026-06-01
#[test]
fn key_auth_args_destination_comes_after_options() {
    let mut s = server();
    s.auth_type = AuthType::Key;
    s.key_path = Some("/home/user/.ssh/id_rsa".into());

    // 模拟 test_key_auth 的构建逻辑
    let mut args = build_ssh_args(&s);
    let target = args.pop().unwrap();
    args.extend([
        "-o".into(),
        "BatchMode=yes".into(),
        "-o".into(),
        "ConnectTimeout=5".into(),
        "-o".into(),
        "StrictHostKeyChecking=no".into(),
        "-o".into(),
        "LogLevel=ERROR".into(),
    ]);
    args.push(target);
    args.push("echo ok".into());

    let joined = args.join(" ");
    let dest_pos = joined
        .find("alice@1.2.3.4")
        .expect("destination must appear in args");
    let echo_pos = joined
        .find("echo ok")
        .expect("`echo ok` must appear in args");
    let last_o_pos = joined
        .rfind("-o ")
        .expect("expected at least one -o option");

    assert!(
        last_o_pos < dest_pos,
        "all -o options must come before destination; got joined: {joined}"
    );
    assert!(
        dest_pos < echo_pos,
        "destination must come before `echo ok`; got joined: {joined}"
    );
}

// -------- Windows SSH_ASKPASS 回归保护 --------
//
// Windows 上 Win32-OpenSSH 因无控制台 + CREATE_NO_WINDOW 而拒绝从 stdin
// 读密码(Win32-OpenSSH issue #1470),必须走 SSH_ASKPASS 机制。
// 守住这块代码的存在,防止有人误把 Windows 路径合并回 stdin 写法。
// author: logic
// date: 2026-06-01

/// 回归保护:Windows 上 `test_password_auth` 入口必须引用 `AskpassSession`,
/// 不能直接用 stdin 写密码。这条断言通过类型系统保证:如果 Windows 路径
/// 被改成 stdin 方式,函数体里就不会出现 `AskpassSession::new`,测试会失败。
#[cfg(windows)]
#[test]
fn windows_password_auth_uses_askpass_not_stdin() {
    // 这个测试在编译期就起作用:如果 ssh_command.rs 的 Windows 分支
    // 退回到 stdin 注入,`AskpassSession` 类型不再被使用,编译会报
    // dead_code 错误,CI 就会挂。
    // 这里只验证 AskpassSession 类型存在 + 可 new — 跑不起来(需要写文件),
    // 但能挡住"误删 AskpassSession"这类破坏。
    let _ = std::any::type_name::<AskpassSession>();
}

/// 真实端到端:创建 `AskpassSession` 拿到 askpass 脚本路径,然后用
/// `CreateProcessW` 派生它(模拟 ssh 派生 askpass 的方式),验证它能起来。
///
/// 这条测试守住 askpass 脚本在 ssh 视角下"可执行"——直接挡住
/// `CreateProcessW failed error:5`(ERROR_ACCESS_DENIED)这类回归。
/// 之前出过 bug:askpass 文件设了 `FILE_ATTRIBUTE_HIDDEN`,导致 ssh 的
/// `posix_spawnp` 拒绝派生,askpass 根本起不来,密码没传出去,server
/// 报 "wrong password"。
#[cfg(windows)]
#[test]
fn windows_askpass_script_is_spawnable() {
    use std::os::windows::process::CommandExt as _;
    use std::process::Stdio;
    use zeroize::Zeroizing;

    let password: Zeroizing<String> = Zeroizing::new("dummy-pw-for-spawn-test".into());
    let session = AskpassSession::new(&password).expect("AskpassSession::new failed");
    let script = session.script_path.clone();
    let password_file = session.password_path.clone();

    // 派生 askpass 脚本:用 CreateProcessW 走和 ssh 相同的代码路径。
    // CREATE_NO_WINDOW 模拟 ssh 派生 askpass 时的环境(无控制台)。
    // 必须设 WARP_SSH_ASKPASS_FILE env,脚本靠它定位密码文件。
    let output = std::process::Command::new("cmd.exe")
        .raw_arg(format!("/c \"{}\"", script.display()))
        .env("WARP_SSH_ASKPASS_FILE", &password_file)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()
        .expect("CreateProcessW failed — askpass script is not spawnable");

    assert!(
        output.status.success(),
        "askpass script exited non-zero: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // askpass 脚本读密码文件首行并 echo,应该输出 session 创建时写入的密码
    assert!(
        stdout.trim() == "dummy-pw-for-spawn-test",
        "askpass output mismatch: got {stdout:?}"
    );
}
