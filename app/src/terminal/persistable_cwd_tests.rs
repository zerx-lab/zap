use super::persistable_local_cwd;

#[test]
fn prefers_verified_local_path() {
    assert_eq!(
        persistable_local_cwd(
            Some("/tmp/verified".to_string()),
            Some(true),
            Some("/tmp/metadata".to_string()),
        ),
        Some("/tmp/verified".to_string())
    );
}

#[test]
fn falls_back_to_metadata_pwd_when_local_session_lacks_verified_path() {
    assert_eq!(
        persistable_local_cwd(None, Some(true), Some("/tmp/metadata".to_string())),
        Some("/tmp/metadata".to_string())
    );
}

#[test]
fn falls_back_to_metadata_pwd_when_session_is_unknown() {
    assert_eq!(
        persistable_local_cwd(None, None, Some("/tmp/metadata".to_string())),
        Some("/tmp/metadata".to_string())
    );
}

#[test]
fn does_not_persist_remote_session_pwd() {
    assert_eq!(
        persistable_local_cwd(None, Some(false), Some("/remote/home".to_string())),
        None
    );
}

#[test]
fn returns_none_when_no_path_is_available() {
    assert_eq!(persistable_local_cwd(None, Some(true), None), None);
    assert_eq!(persistable_local_cwd(None, None, None), None);
}
