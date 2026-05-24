//! OS keychain 封装 — 用 `keyring` crate 跨平台。
//! Windows = Credential Manager / macOS = Keychain / Linux = Secret Service。
//!
//! Account key 形如 `<node_uuid>:password` 或 `<node_uuid>:passphrase`,
//! 不依赖 host/username,**节点改名或改 host 不丢密码**。

use thiserror::Error;
use zeroize::Zeroizing;

const SERVICE: &str = "zap.ssh";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SecretKind {
    Password,
    Passphrase,
    RootPassword,
}

impl SecretKind {
    fn suffix(&self) -> &'static str {
        match self {
            SecretKind::Password => "password",
            SecretKind::Passphrase => "passphrase",
            SecretKind::RootPassword => "root_password",
        }
    }
}

#[derive(Debug, Error)]
pub enum SshSecretStoreError {
    /// 平台没有可用的 keychain backend(常见于 Linux headless / WSL 无
    /// Secret Service)。UI 层应提示用户改用私钥。
    #[error("no keychain backend available on this platform")]
    NoBackend,
    #[error("keyring error: {0}")]
    Keyring(String),
}

impl From<keyring::Error> for SshSecretStoreError {
    fn from(e: keyring::Error) -> Self {
        match e {
            keyring::Error::NoStorageAccess(_) | keyring::Error::PlatformFailure(_) => {
                SshSecretStoreError::NoBackend
            }
            other => SshSecretStoreError::Keyring(other.to_string()),
        }
    }
}

/// 凭据存储抽象 — `KeychainSecretStore` 是默认实现,测试可用 mock。
pub trait SshSecretStore: Send + Sync {
    fn set(&self, node_id: &str, kind: SecretKind, secret: &str)
    -> Result<(), SshSecretStoreError>;

    fn get(
        &self,
        node_id: &str,
        kind: SecretKind,
    ) -> Result<Option<Zeroizing<String>>, SshSecretStoreError>;

    fn delete(&self, node_id: &str, kind: SecretKind) -> Result<(), SshSecretStoreError>;
}

#[derive(Default, Clone, Copy, Debug)]
pub struct KeychainSecretStore;

fn account_key(node_id: &str, kind: SecretKind) -> String {
    format!("{node_id}:{}", kind.suffix())
}

impl SshSecretStore for KeychainSecretStore {
    fn set(
        &self,
        node_id: &str,
        kind: SecretKind,
        secret: &str,
    ) -> Result<(), SshSecretStoreError> {
        let entry = keyring::Entry::new(SERVICE, &account_key(node_id, kind))?;
        entry.set_password(secret)?;
        Ok(())
    }

    fn get(
        &self,
        node_id: &str,
        kind: SecretKind,
    ) -> Result<Option<Zeroizing<String>>, SshSecretStoreError> {
        let entry = keyring::Entry::new(SERVICE, &account_key(node_id, kind))?;
        match entry.get_password() {
            Ok(s) => Ok(Some(Zeroizing::new(s))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn delete(&self, node_id: &str, kind: SecretKind) -> Result<(), SshSecretStoreError> {
        let entry = keyring::Entry::new(SERVICE, &account_key(node_id, kind))?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    //! 进程内的内存 mock,绕开 OS keychain — CI / 单测用。

    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    pub struct InMemorySecretStore {
        inner: Mutex<HashMap<String, String>>,
    }

    impl SshSecretStore for InMemorySecretStore {
        fn set(
            &self,
            node_id: &str,
            kind: SecretKind,
            secret: &str,
        ) -> Result<(), SshSecretStoreError> {
            self.inner
                .lock()
                .unwrap()
                .insert(account_key(node_id, kind), secret.to_string());
            Ok(())
        }

        fn get(
            &self,
            node_id: &str,
            kind: SecretKind,
        ) -> Result<Option<Zeroizing<String>>, SshSecretStoreError> {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .get(&account_key(node_id, kind))
                .cloned()
                .map(Zeroizing::new))
        }

        fn delete(&self, node_id: &str, kind: SecretKind) -> Result<(), SshSecretStoreError> {
            self.inner
                .lock()
                .unwrap()
                .remove(&account_key(node_id, kind));
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::InMemorySecretStore;
    use super::*;

    #[test]
    fn set_get_delete_roundtrip() {
        let store = InMemorySecretStore::default();
        let node = "abc-123";
        store.set(node, SecretKind::Password, "hunter2").unwrap();
        let got = store.get(node, SecretKind::Password).unwrap().unwrap();
        assert_eq!(&*got, "hunter2");
        store.delete(node, SecretKind::Password).unwrap();
        assert!(store.get(node, SecretKind::Password).unwrap().is_none());
    }

    #[test]
    fn password_and_passphrase_have_separate_keys() {
        let store = InMemorySecretStore::default();
        store.set("n", SecretKind::Password, "pw").unwrap();
        store.set("n", SecretKind::Passphrase, "pp").unwrap();
        assert_eq!(
            &*store.get("n", SecretKind::Password).unwrap().unwrap(),
            "pw"
        );
        assert_eq!(
            &*store.get("n", SecretKind::Passphrase).unwrap().unwrap(),
            "pp"
        );
    }

    #[test]
    fn delete_missing_is_idempotent() {
        let store = InMemorySecretStore::default();
        // Should not error on absent key.
        store.delete("never-stored", SecretKind::Password).unwrap();
    }
}
