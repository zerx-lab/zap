use super::bytes_look_like_shell_prompt;

fn matches(input: &str) -> bool {
    bytes_look_like_shell_prompt(input.as_bytes())
}

#[test]
fn matches_dollar_prompt() {
    assert!(matches("user@host:~$ "));
    assert!(matches("$ "));
}

#[test]
fn matches_hash_root_prompt() {
    assert!(matches("root@host:~# "));
    assert!(matches("# "));
}

#[test]
fn matches_powershell_prompt() {
    assert!(matches("PS C:\\Users\\u> "));
    assert!(matches("> "));
}

#[test]
fn matches_powerline_prompts() {
    assert!(matches("❯ "));
    assert!(matches("▶ "));
    assert!(matches("» "));
    assert!(matches("λ "));
    assert!(matches("→ "));
}

#[test]
fn does_not_match_partial_prompt_chars() {
    // 缺空格不算 prompt
    assert!(!matches("$"));
    assert!(!matches("#"));
    assert!(!matches(">"));
    assert!(!matches("❯"));
}

#[test]
fn does_not_match_random_output() {
    assert!(!matches("hello world"));
    assert!(!matches("error: connection refused\n"));
}

#[test]
fn matches_with_long_preceding_output() {
    // tail 只看 256 字节,前面有 1KB 输出,只要末尾是 prompt 仍命中
    let mut s = "x".repeat(1024);
    s.push_str("$ ");
    assert!(matches(&s));
}

#[test]
fn does_not_match_quoted_prompt_in_middle() {
    // prompt 字符出现在末尾以外位置不应该误命中
    assert!(!matches("$ foo"));
    assert!(!matches("# comment"));
}
