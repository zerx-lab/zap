//! zap_sftp::types 模块单元测试
//!
//! author: logic
//! date: 2026/05/26

use zap_sftp::types::*;

// ============================================================
// FileType::from_mode 测试
// ============================================================

/// 验证 0o040000 解析为 Dir
#[test]
fn test_file_type_from_mode_dir() {
    let ft = FileType::from_mode(0o040000);
    assert_eq!(ft, FileType::Dir);
}

/// 验证 0o100000 解析为 File
#[test]
fn test_file_type_from_mode_file() {
    let ft = FileType::from_mode(0o100000);
    assert_eq!(ft, FileType::File);
}

/// 验证 0o120000 解析为 Symlink
#[test]
fn test_file_type_from_mode_symlink() {
    let ft = FileType::from_mode(0o120000);
    assert_eq!(ft, FileType::Symlink);
}

/// 验证 0o000000 解析为 Other
#[test]
fn test_file_type_from_mode_other() {
    let ft = FileType::from_mode(0o000000);
    assert_eq!(ft, FileType::Other);
}

/// 验证未知类型 0o050000 也解析为 Other
#[test]
fn test_file_type_from_mode_unknown() {
    let ft = FileType::from_mode(0o050000);
    assert_eq!(ft, FileType::Other);
}

// ============================================================
// FilePermissions::from_mode 测试
// ============================================================

/// 验证 0o755 => rwxr-xr-x
#[test]
fn test_file_permissions_from_mode_755() {
    let p = FilePermissions::from_mode(0o755);
    assert!(p.owner_read, "owner_read 应为 true");
    assert!(p.owner_write, "owner_write 应为 true");
    assert!(p.owner_exec, "owner_exec 应为 true");
    assert!(p.group_read, "group_read 应为 true");
    assert!(!p.group_write, "group_write 应为 false");
    assert!(p.group_exec, "group_exec 应为 true");
    assert!(p.other_read, "other_read 应为 true");
    assert!(!p.other_write, "other_write 应为 false");
    assert!(p.other_exec, "other_exec 应为 true");
}

/// 验证 0o644 => rw-r--r--
#[test]
fn test_file_permissions_from_mode_644() {
    let p = FilePermissions::from_mode(0o644);
    assert!(p.owner_read, "owner_read 应为 true");
    assert!(p.owner_write, "owner_write 应为 true");
    assert!(!p.owner_exec, "owner_exec 应为 false");
    assert!(p.group_read, "group_read 应为 true");
    assert!(!p.group_write, "group_write 应为 false");
    assert!(!p.group_exec, "group_exec 应为 false");
    assert!(p.other_read, "other_read 应为 true");
    assert!(!p.other_write, "other_write 应为 false");
    assert!(!p.other_exec, "other_exec 应为 false");
}

/// 验证 0o777 => 所有位均为 true
#[test]
fn test_file_permissions_from_mode_777() {
    let p = FilePermissions::from_mode(0o777);
    assert!(
        p.owner_read && p.owner_write && p.owner_exec,
        "owner 位应全部为 true"
    );
    assert!(
        p.group_read && p.group_write && p.group_exec,
        "group 位应全部为 true"
    );
    assert!(
        p.other_read && p.other_write && p.other_exec,
        "other 位应全部为 true"
    );
}

/// 验证 0o000 => 所有位均为 false
#[test]
fn test_file_permissions_from_mode_000() {
    let p = FilePermissions::from_mode(0o000);
    assert!(
        !p.owner_read && !p.owner_write && !p.owner_exec,
        "owner 位应全部为 false"
    );
    assert!(
        !p.group_read && !p.group_write && !p.group_exec,
        "group 位应全部为 false"
    );
    assert!(
        !p.other_read && !p.other_write && !p.other_exec,
        "other 位应全部为 false"
    );
}

/// 验证 0o111 => 仅执行位为 true
#[test]
fn test_file_permissions_from_mode_exec_only() {
    let p = FilePermissions::from_mode(0o111);
    assert!(!p.owner_read, "owner_read 应为 false");
    assert!(!p.owner_write, "owner_write 应为 false");
    assert!(p.owner_exec, "owner_exec 应为 true");
    assert!(!p.group_read, "group_read 应为 false");
    assert!(!p.group_write, "group_write 应为 false");
    assert!(p.group_exec, "group_exec 应为 true");
    assert!(!p.other_read, "other_read 应为 false");
    assert!(!p.other_write, "other_write 应为 false");
    assert!(p.other_exec, "other_exec 应为 true");
}

// ============================================================
// OpenOptions 构造器测试
// ============================================================

/// 验证 read() 构造的 OpenOptions 字段值
#[test]
fn test_open_options_read() {
    let opts = OpenOptions::read();
    assert!(opts.read, "read 应为 true");
    assert!(opts.write.is_none(), "write 应为 None");
    assert!(!opts.create, "create 应为 false");
    assert!(!opts.truncate, "truncate 应为 false");
    assert_eq!(opts.file_type, OpenFileType::File);
}

/// 验证 write() 构造的 OpenOptions 字段值
#[test]
fn test_open_options_write() {
    let opts = OpenOptions::write();
    assert!(!opts.read, "read 应为 false");
    assert_eq!(opts.write, Some(WriteMode::Write), "write 应为 Some(Write)");
    assert!(opts.create, "create 应为 true");
    assert!(opts.truncate, "truncate 应为 true");
    assert_eq!(opts.mode, Some(0o644), "mode 应为 Some(0o644)");
    assert_eq!(opts.file_type, OpenFileType::File);
}

/// 验证 append() 构造的 OpenOptions 字段值
#[test]
fn test_open_options_append() {
    let opts = OpenOptions::append();
    assert!(!opts.read, "read 应为 false");
    assert_eq!(
        opts.write,
        Some(WriteMode::Append),
        "write 应为 Some(Append)"
    );
    assert!(opts.create, "create 应为 true");
    assert!(!opts.truncate, "truncate 应为 false");
}

/// 验证 create_new() 构造的 OpenOptions 字段值
#[test]
fn test_open_options_create_new() {
    let opts = OpenOptions::create_new();
    assert!(!opts.read, "read 应为 false");
    assert_eq!(opts.write, Some(WriteMode::Write), "write 应为 Some(Write)");
    assert!(opts.create, "create 应为 true");
    assert!(!opts.truncate, "truncate 应为 false");
}

// ============================================================
// RenameOptions Default 测试
// ============================================================

/// 验证 RenameOptions 默认值全部为 false
#[test]
fn test_rename_options_default() {
    let opts = RenameOptions::default();
    assert!(!opts.overwrite, "overwrite 应为 false");
    assert!(!opts.atomic, "atomic 应为 false");
    assert!(!opts.native, "native 应为 false");
}
