use std::ffi::OsStr;

use command::blocking::Command;

use super::apply_color_support_env;

fn env_value<'a>(cmd: &'a Command, key: &str) -> Option<Option<&'a OsStr>> {
    cmd.get_envs()
        .find(|(k, _)| *k == OsStr::new(key))
        .map(|(_, v)| v)
}

#[test]
fn apply_color_support_env_advertises_truecolor() {
    let mut cmd = Command::new("true");
    apply_color_support_env(&mut cmd);

    assert_eq!(
        env_value(&cmd, "COLORTERM"),
        Some(Some(OsStr::new("truecolor")))
    );
    assert_eq!(env_value(&cmd, "FORCE_COLOR"), Some(Some(OsStr::new("3"))));
    assert_eq!(
        env_value(&cmd, "CLICOLOR_FORCE"),
        Some(Some(OsStr::new("1")))
    );
}

#[test]
fn apply_color_support_env_strips_inherited_color_killers() {
    let mut cmd = Command::new("true");
    cmd.env("NO_COLOR", "1");
    cmd.env("NODE_DISABLE_COLORS", "1");
    cmd.env("FORCE_COLOR", "0");
    apply_color_support_env(&mut cmd);

    // env_remove 在 get_envs 里表现为 value=None,不再从父进程继承。
    assert_eq!(env_value(&cmd, "NO_COLOR"), Some(None));
    assert_eq!(env_value(&cmd, "NODE_DISABLE_COLORS"), Some(None));
    assert_eq!(env_value(&cmd, "FORCE_COLOR"), Some(Some(OsStr::new("3"))));
}

#[test]
fn apply_color_support_env_overrides_caller_no_color_after_env_vars() {
    let mut cmd = Command::new("true");
    // 模拟 env_vars 循环把禁色变量写回来之后再 sanitize。
    cmd.env("NO_COLOR", "1");
    cmd.env("FORCE_COLOR", "0");
    apply_color_support_env(&mut cmd);

    assert_eq!(env_value(&cmd, "NO_COLOR"), Some(None));
    assert_eq!(env_value(&cmd, "FORCE_COLOR"), Some(Some(OsStr::new("3"))));
}

#[test]
fn apply_color_support_env_strips_inherited_git_pager_override() {
    let mut cmd = Command::new("true");
    cmd.env("GIT_CONFIG_COUNT", "1");
    apply_color_support_env(&mut cmd);

    assert_eq!(env_value(&cmd, "GIT_CONFIG_COUNT"), Some(None));
}
