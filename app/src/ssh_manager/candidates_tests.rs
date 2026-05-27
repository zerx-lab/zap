//! `candidates::CandidatesViewModel::rows()` 的纯函数单测。
//!
//! 这里只验"状态 → 行列表"的映射;真实 IO(`load_candidates`)、warpui
//! runtime 都不参与。覆盖矩阵:状态 5 种(未加载 / NotFound / Error /
//! Empty / 非空 Loaded)× 折叠 2 种 × `added_aliases` 命中/未命中。

use std::collections::HashSet;
use std::path::PathBuf;

use warp_ssh_manager::SshConfigCandidate;

use super::{
    CandidateRow, CandidatesViewModel, fake_load_result_error, fake_load_result_loaded,
    fake_load_result_not_found,
};

fn cand(alias: &str) -> SshConfigCandidate {
    SshConfigCandidate {
        alias: alias.into(),
        hostname: None,
        user: None,
        port: None,
        identity_file: None,
    }
}

fn full_cand() -> SshConfigCandidate {
    SshConfigCandidate {
        alias: "prodbox".into(),
        hostname: Some("prod.example.com".into()),
        user: Some("alice".into()),
        port: Some(2222),
        identity_file: Some(PathBuf::from("/home/alice/.ssh/id_ed25519")),
    }
}

#[test]
fn rows_when_state_is_none_is_empty() {
    // 模型刚创建、还没 refresh —— panel 据此完全不渲染区段。
    let vm = CandidatesViewModel::new();
    assert_eq!(vm.rows(), Vec::<CandidateRow>::new());
}

#[test]
fn rows_when_not_found_returns_header_plus_not_found() {
    let vm = CandidatesViewModel::with_state(
        Some(fake_load_result_not_found("/home/u/.ssh/config")),
        HashSet::new(),
        true,
    );
    let rows = vm.rows();
    assert_eq!(rows.len(), 2);
    assert!(matches!(
        rows[0],
        CandidateRow::Header {
            count: 0,
            can_refresh: true,
            ..
        }
    ));
    match &rows[1] {
        CandidateRow::NotFound { path_display } => {
            assert_eq!(path_display, "/home/u/.ssh/config");
        }
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn rows_when_error_returns_header_plus_error_with_message() {
    let vm = CandidatesViewModel::with_state(
        Some(fake_load_result_error(
            "/home/u/.ssh/config",
            "permission denied (os error 13)",
        )),
        HashSet::new(),
        true,
    );
    let rows = vm.rows();
    assert_eq!(rows.len(), 2);
    match &rows[1] {
        CandidateRow::Error {
            path_display,
            message,
        } => {
            assert_eq!(path_display, "/home/u/.ssh/config");
            assert_eq!(message, "permission denied (os error 13)");
        }
        other => panic!("expected Error, got {other:?}"),
    }
}

#[test]
fn rows_when_loaded_empty_returns_header_plus_empty() {
    let vm = CandidatesViewModel::with_state(
        Some(fake_load_result_loaded("/home/u/.ssh/config", vec![])),
        HashSet::new(),
        true,
    );
    let rows = vm.rows();
    assert_eq!(rows.len(), 2);
    assert!(matches!(rows[0], CandidateRow::Header { count: 0, .. }));
    assert!(matches!(rows[1], CandidateRow::Empty { .. }));
}

#[test]
fn rows_when_loaded_non_empty_returns_header_plus_one_per_candidate() {
    let vm = CandidatesViewModel::with_state(
        Some(fake_load_result_loaded(
            "/home/u/.ssh/config",
            vec![cand("a"), cand("b"), cand("c")],
        )),
        HashSet::new(),
        true,
    );
    let rows = vm.rows();
    assert_eq!(rows.len(), 4);
    match &rows[0] {
        CandidateRow::Header { count, .. } => assert_eq!(*count, 3),
        other => panic!("expected Header, got {other:?}"),
    }
    let aliases: Vec<&str> = rows[1..]
        .iter()
        .map(|r| match r {
            CandidateRow::Candidate { alias, .. } => alias.as_str(),
            other => panic!("expected Candidate, got {other:?}"),
        })
        .collect();
    assert_eq!(aliases, vec!["a", "b", "c"]);
}

#[test]
fn rows_marks_added_when_alias_in_added_set() {
    // PRODUCT.md decision E:已导入 → `added=true`,UI 显示 "Added" 徽章。
    let mut added = HashSet::new();
    added.insert("b".to_string());
    let vm = CandidatesViewModel::with_state(
        Some(fake_load_result_loaded(
            "/home/u/.ssh/config",
            vec![cand("a"), cand("b"), cand("c")],
        )),
        added,
        true,
    );
    let rows = vm.rows();
    let marks: Vec<bool> = rows[1..]
        .iter()
        .map(|r| match r {
            CandidateRow::Candidate { added, .. } => *added,
            other => panic!("expected Candidate, got {other:?}"),
        })
        .collect();
    assert_eq!(marks, vec![false, true, false]);
}

#[test]
fn rows_propagates_all_candidate_fields() {
    let vm = CandidatesViewModel::with_state(
        Some(fake_load_result_loaded(
            "/home/u/.ssh/config",
            vec![full_cand()],
        )),
        HashSet::new(),
        true,
    );
    let rows = vm.rows();
    match &rows[1] {
        CandidateRow::Candidate {
            alias,
            hostname,
            user,
            port,
            identity_file,
            added,
        } => {
            assert_eq!(alias, "prodbox");
            assert_eq!(hostname.as_deref(), Some("prod.example.com"));
            assert_eq!(user.as_deref(), Some("alice"));
            assert_eq!(*port, Some(2222));
            // PathBuf::display() 在跨平台上分隔符可能不同 —— 用 contains 而不是
            // 字面相等,断言关键路径段在里面就够。
            assert!(identity_file.as_deref().unwrap().contains("id_ed25519"));
            assert!(!*added);
        }
        other => panic!("expected Candidate, got {other:?}"),
    }
}

#[test]
fn rows_when_collapsed_returns_header_only() {
    // 折叠后 body(NotFound / Empty / Error / Candidate) 全部不渲染。
    let vm = CandidatesViewModel::with_state(
        Some(fake_load_result_loaded(
            "/home/u/.ssh/config",
            vec![cand("a"), cand("b")],
        )),
        HashSet::new(),
        false,
    );
    let rows = vm.rows();
    assert_eq!(rows.len(), 1);
    match &rows[0] {
        CandidateRow::Header { count, .. } => assert_eq!(*count, 2),
        other => panic!("expected Header, got {other:?}"),
    }
}

#[test]
fn find_candidate_returns_match_by_alias() {
    let vm = CandidatesViewModel::with_state(
        Some(fake_load_result_loaded(
            "/home/u/.ssh/config",
            vec![cand("a"), full_cand(), cand("c")],
        )),
        HashSet::new(),
        true,
    );
    let got = vm.find_candidate("prodbox").expect("present");
    assert_eq!(got.hostname.as_deref(), Some("prod.example.com"));
    assert_eq!(got.port, Some(2222));
    assert!(vm.find_candidate("does-not-exist").is_none());
}

#[test]
fn path_display_reflects_state_path() {
    let vm = CandidatesViewModel::with_state(
        Some(fake_load_result_loaded("/etc/ssh/config", vec![])),
        HashSet::new(),
        true,
    );
    assert_eq!(vm.path_display().as_deref(), Some("/etc/ssh/config"));

    // 状态为空时 path_display 返回 None
    let empty = CandidatesViewModel::new();
    assert!(empty.path_display().is_none());
}
