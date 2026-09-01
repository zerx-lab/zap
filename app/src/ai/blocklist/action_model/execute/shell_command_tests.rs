use std::time::Duration;

use super::*;

#[test]
fn detects_interactive_session_commands_across_platforms() {
    for command in [
        "ssh root@example.com",
        "command ssh localhost",
        "ssh.exe -p 2222 root@example.com",
        "/usr/bin/ssh host",
        r#""C:\Windows\System32\OpenSSH\ssh.exe" -p 22 host"#,
        r#"& "C:\Program Files\OpenSSH\ssh.exe" host"#,
        "warp_run_generator_command 42 'ssh host'",
        " warp_run_generator_command 42 'ssh host'",
        "Zap-Run-GeneratorCommand 42 'ssh host' -ErrorAction Ignore",
        r#"warp_run_generator_command 42 '"C:\Windows\System32\OpenSSH\ssh.exe" host'"#,
        "gcloud compute ssh --zone us-west1-a my-instance",
        "eb ssh --profile my-profile my-env",
        "doctl compute ssh --region nyc1 my-droplet",
        "mosh root@example.com",
        "sftp root@example.com",
        "telnet example.com",
    ] {
        assert_eq!(
            command_starts_non_terminating_session(command),
            true,
            "{command}"
        );
    }
}

#[test]
fn does_not_detect_unrelated_or_non_interactive_ssh_commands() {
    for command in [
        "",
        "echo ssh",
        "git status",
        "ssh-add-key",
        "ssh -T user@host",
        "ssh -v user@host -W localhost:22",
        "ssh user@host ls",
        "ssh.exe user@host ls",
        r#""C:\Windows\System32\OpenSSH\ssh.exe" user@host ls"#,
        r#"& "C:\Program Files\OpenSSH\ssh.exe" user@host ls"#,
        "warp_run_generator_command 42 'ssh user@host ls'",
        "Zap-Run-GeneratorCommand 42 'git status' -ErrorAction Ignore",
        "rsync myfile.txt ssh://user@server.com",
        // 右引号后还粘着字符,故意拒绝 tokenize,避免被错切成 `ssh`
        // 然后通过 `ssh hello-world` 误判为交互会话。
        r#""ssh"hello-world"#,
        // 未闭合的引号同样拒绝 tokenize。
        r#""ssh hello world"#,
    ] {
        assert_eq!(
            command_starts_non_terminating_session(command),
            false,
            "{command}"
        );
    }
}

#[test]
fn shortens_on_completion_delay_for_interactive_sessions() {
    assert_eq!(
        effective_read_shell_command_delay("ssh host", Some(ShellCommandDelay::OnCompletion)),
        ActionResultDelay::OnCompletion {
            timeout: ShellCommandExecutor::MAX_WAIT_DURATION
        }
    );
    assert_eq!(
        effective_read_shell_command_delay(
            r#"& "C:\Program Files\OpenSSH\ssh.exe" host"#,
            Some(ShellCommandDelay::OnCompletion)
        ),
        ActionResultDelay::OnCompletion {
            timeout: ShellCommandExecutor::MAX_WAIT_DURATION
        }
    );
    assert_eq!(
        effective_read_shell_command_delay(
            "warp_run_generator_command 42 'ssh host'",
            Some(ShellCommandDelay::OnCompletion)
        ),
        ActionResultDelay::OnCompletion {
            timeout: ShellCommandExecutor::MAX_WAIT_DURATION
        }
    );
    assert_eq!(
        effective_read_shell_command_delay("mosh host", None),
        ActionResultDelay::OnCompletion {
            timeout: ShellCommandExecutor::MAX_WAIT_DURATION
        }
    );
}

#[test]
fn preserves_explicit_or_non_interactive_read_delays() {
    assert_eq!(
        effective_read_shell_command_delay(
            "ssh host",
            Some(ShellCommandDelay::Duration(Duration::from_secs(8)))
        ),
        ActionResultDelay::Duration(Duration::from_secs(8))
    );
    assert_eq!(
        effective_read_shell_command_delay("git status", Some(ShellCommandDelay::OnCompletion)),
        ActionResultDelay::OnCompletion {
            timeout: ShellCommandExecutor::MAX_AGENT_DELAY_DURATION
        }
    );
    assert_eq!(
        effective_read_shell_command_delay("git status", None),
        ActionResultDelay::Default
    );
}

#[test]
fn requested_command_wait_until_completion_does_not_use_snapshot_timeout() {
    assert_eq!(
        action_result_delay_for_requested_command(true),
        ActionResultDelay::UntilCompletion
    );
    assert_eq!(
        action_result_delay_for_requested_command(false),
        ActionResultDelay::Default
    );
}

#[test]
fn preemption_logic_covers_until_completion_timeout() {
    use ActionResultDelay::{Default, Duration as DurationDelay, OnCompletion, UntilCompletion};
    use WakeReason::*;

    // BlockFinished 从不抢占 —— 它是“命令真正完成”的信号。
    assert!(!compute_is_preempted(BlockFinished, UntilCompletion));
    assert!(!compute_is_preempted(BlockFinished, Default));
    assert!(!compute_is_preempted(
        BlockFinished,
        OnCompletion {
            timeout: Duration::from_secs(1)
        }
    ));

    // ForceRefresh 总是抢占,与 delay 无关。
    assert!(compute_is_preempted(ForceRefresh, UntilCompletion));
    assert!(compute_is_preempted(ForceRefresh, Default));

    // Timeout + OnCompletion / UntilCompletion 是抢占。
    assert!(compute_is_preempted(
        Timeout,
        OnCompletion {
            timeout: Duration::from_secs(1)
        }
    ));
    // #138: pager 卡死兜底超时必须被标记为抢占,避免 server 误解为“命令完成”。
    assert!(compute_is_preempted(Timeout, UntilCompletion));

    // Timeout + Default / Duration 不是抢占 —— agent 本来就预期会拿到中间快照。
    assert!(!compute_is_preempted(Timeout, Default));
    assert!(!compute_is_preempted(
        Timeout,
        DurationDelay(Duration::from_secs(1))
    ));
}

#[test]
fn detects_git_log_stat_as_implicit_pager_command() {
    for command in [
        "git log --stat",
        "git log",
        "git diff",
        "git show HEAD",
        "git blame README.md",
        "git reflog",
        "man git-log",
        "less CHANGELOG.md",
        "/usr/bin/git log --stat",
        "command git log --stat",
        r#""C:\Program Files\Git\cmd\git.exe" log --stat"#,
        "warp_run_generator_command 42 'git log --stat'",
        "cd /tmp && git log --stat",
        "cd /tmp; git log --stat",
        "noglob git log --stat",
        "PAGER=less git log --stat",
        "git log --pretty=format:'%h | %s' --stat",
        "git log --stat | less",
    ] {
        assert_eq!(command_uses_implicit_pager(command), true, "{command}");
    }
}

#[test]
fn does_not_treat_non_pager_or_explicitly_disabled_git_as_pager() {
    for command in [
        "git status",
        "git log --stat | cat",
        "cd /tmp && git log --stat | cat",
        "git --no-pager log --stat",
        "git -P log --stat",
        "git log --no-pager --stat",
        "git commit -m 'log --stat'",
        "echo git log --stat",
        "ls",
        "cd /tmp && git status",
    ] {
        assert_eq!(command_uses_implicit_pager(command), false, "{command}");
    }
}

#[test]
fn requested_command_keeps_pager_for_interactive_git_log() {
    assert!(!should_disable_pager_for_requested_command(
        true,
        "git log --stat"
    ));
    assert!(!should_disable_pager_for_requested_command(
        false,
        "git log --stat"
    ));
    assert!(should_disable_pager_for_requested_command(
        true,
        "git status"
    ));
    assert!(!should_disable_pager_for_requested_command(
        false,
        "git status"
    ));
    assert!(!should_disable_pager_for_requested_command(
        true,
        "cd /tmp && git log --stat"
    ));
    assert!(!should_disable_pager_for_requested_command(
        true,
        "git log --pretty=format:'%h | %s' --stat"
    ));
    assert!(should_disable_pager_for_requested_command(
        true,
        "git log --stat | cat"
    ));
}
