use super::{PASSWORD_PROMPT_REGEX, SU_ROOT_CMD_REGEX, is_su_to_root};

fn pw_matches(input: &str) -> bool {
    PASSWORD_PROMPT_REGEX.is_match(input.as_bytes())
}

fn su_matches(input: &str) -> bool {
    SU_ROOT_CMD_REGEX.is_match(input.as_bytes())
}

#[test]
fn password_prompt_matches_typical_forms() {
    // 半角冒号
    assert!(pw_matches("Password:"));
    assert!(pw_matches("Password: "));
    assert!(pw_matches("[sudo] password for alice: "));
    assert!(pw_matches("user@host's password: "));
    // 全角冒号(中文输入法)
    assert!(pw_matches("密码:"));
    assert!(pw_matches("密码："));
    // 银河麒麟 V10 无冒号特例
    assert!(pw_matches("输入密码"));
    assert!(pw_matches("输入密码 "));
    // passphrase
    assert!(pw_matches("Enter passphrase for key '/home/u/.ssh/id_rsa': "));
}

#[test]
fn password_prompt_rejects_false_positives() {
    // 这些都是含 'password' / '密码' 但不是真正提示的输出,不能假阳性
    assert!(!pw_matches("Your password has expired"));
    assert!(!pw_matches("Bad password, try again"));
    assert!(!pw_matches("password changed successfully"));
    assert!(!pw_matches("New password for root"));
    assert!(!pw_matches("Welcome! Please change your password soon.\n"));
    assert!(!pw_matches("Last login: Mon Jan 1 password rotated yesterday\n"));
    // 中文同理
    assert!(!pw_matches("您的密码已过期"));
}

#[test]
fn su_root_matches_common_variants() {
    // 最基本
    assert!(su_matches("su"));
    assert!(su_matches("su\n"));
    // 不带用户名的快捷形式(默认 root)
    assert!(su_matches("su -"));
    assert!(su_matches("su -l"));
    assert!(su_matches("su --login"));
    // 显式 root
    assert!(su_matches("su root"));
    assert!(su_matches("su - root"));
    assert!(su_matches("su -l root"));
    assert!(su_matches("su --login root"));
    // sudo su(\bsu 仍能命中)
    assert!(su_matches("sudo su"));
}

#[test]
fn su_to_other_user_does_not_match() {
    // 切到非 root 用户不应触发
    assert!(!su_matches("su lg"));
    assert!(!su_matches("su - lg"));
    assert!(!su_matches("su -l lg"));
    assert!(!su_matches("su --login lg"));
    assert!(!su_matches("su admin"));
}

#[test]
fn su_in_middle_of_other_command_does_not_match() {
    // su 不在行尾不应触发
    assert!(!su_matches("susan"));
    assert!(!su_matches("issue"));
    // grep su file 这种命令,行尾不是 su 也不是 su root 模式
    assert!(!su_matches("grep su /etc/passwd"));
}

#[test]
fn is_su_to_root_detects_in_buffer() {
    let buf = b"user@host:~$ su root\r\nPassword: ";
    assert!(is_su_to_root(buf));

    let buf = b"user@host:~$ su lg\r\nPassword: ";
    assert!(!is_su_to_root(buf));
}

#[test]
fn full_pipeline_su_root_with_password_prompt() {
    // 模拟完整 PTY 序列:用户输入 `su -`,回显后出现密码提示
    let buf = b"alice@kylin:~$ su -\r\n\xe5\xaf\x86\xe7\xa0\x81\xef\xbc\x9a";
    assert!(PASSWORD_PROMPT_REGEX.is_match(buf));
    assert!(is_su_to_root(buf));
}
