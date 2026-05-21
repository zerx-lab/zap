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
        "Warp-Run-GeneratorCommand 42 'ssh host' -ErrorAction Ignore",
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
        "Warp-Run-GeneratorCommand 42 'git status' -ErrorAction Ignore",
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
