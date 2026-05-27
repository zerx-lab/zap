//! Shell prompt 检测。供 SSH 注入器(`secret_injector` / `startup_command_injector`
//! / `su_password_injector`)在等待登录完成、shell 就绪后再触发动作。
//!
//! 仅看 buffer 尾部 256 字节,匹配若干常见 prompt 末尾:
//! - ASCII: `$ ` / `# ` / `> `
//! - 常见 powerline / Starship 符号: ❯  ▶  »  λ  →

const TAIL_BYTES: usize = 256;

/// 检查缓冲区末尾是否匹配 shell prompt 模式。
pub fn bytes_look_like_shell_prompt(bytes: &[u8]) -> bool {
    let tail = if bytes.len() > TAIL_BYTES {
        &bytes[bytes.len() - TAIL_BYTES..]
    } else {
        bytes
    };
    if tail.ends_with(b"$ ") || tail.ends_with(b"# ") || tail.ends_with(b"> ") {
        return true;
    }
    // 多字节 prompt 符号 + 空格
    if tail.ends_with(&[0xe2, 0x9d, 0xaf, 0x20])  // ❯
        || tail.ends_with(&[0xe2, 0x96, 0xb6, 0x20])  // ▶
        || tail.ends_with(&[0xc2, 0xbb, 0x20])  // »
        || tail.ends_with(&[0xce, 0xbb, 0x20])  // λ
        || tail.ends_with(&[0xe2, 0x86, 0x92, 0x20])
    // →
    {
        return true;
    }
    false
}

#[cfg(test)]
#[path = "shell_prompt_tests.rs"]
mod tests;
