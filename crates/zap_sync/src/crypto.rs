//! AES-256-GCM 加密/解密模块
//!
// author: logic
// date: 2026-05-24

use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// 加密/解密错误
#[derive(Debug, Error)]
pub enum CryptoError {
    /// 加密失败
    #[error("加密失败: {0}")]
    Encrypt(String),
    /// 解密失败
    #[error("解密失败: {0}")]
    Decrypt(String),
}

/// 密钥派生固定盐值，与 token 解耦
const KEY_SALT: &[u8] = b"ZAP_SYNC_MASTER_KEY_V1";

/// 派生 32 字节密钥
fn derive_key() -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(KEY_SALT);
    hasher.finalize().into()
}

/// 使用 AES-256-GCM 加密明文，返回 Base64 编码的 nonce+ciphertext
pub fn encrypt(plaintext: &str) -> Result<String, CryptoError> {
    let key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| CryptoError::Encrypt(e.to_string()))?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| CryptoError::Encrypt(e.to_string()))?;
    let mut combined = Vec::with_capacity(12 + ciphertext.len());
    combined.extend_from_slice(&nonce);
    combined.extend_from_slice(&ciphertext);
    Ok(BASE64.encode(&combined))
}

/// 解密 Base64 编码的 nonce+ciphertext
pub fn decrypt(encoded: &str) -> Result<String, CryptoError> {
    let key = derive_key();
    let combined = BASE64
        .decode(encoded)
        .map_err(|e| CryptoError::Decrypt(e.to_string()))?;
    if combined.len() < 12 {
        return Err(CryptoError::Decrypt("数据过短".to_string()));
    }
    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| CryptoError::Decrypt(e.to_string()))?;
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| CryptoError::Decrypt(e.to_string()))?;
    String::from_utf8(plaintext).map_err(|e| CryptoError::Decrypt(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let plaintext = "my_secret_password";
        let encrypted = encrypt(plaintext).unwrap();
        let decrypted = decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_deterministic_key_derivation() {
        let encrypted1 = encrypt("secret").unwrap();
        let decrypted1 = decrypt(&encrypted1).unwrap();
        assert_eq!(decrypted1, "secret");
    }

    #[test]
    fn test_empty_string() {
        let encrypted = encrypt("").unwrap();
        let decrypted = decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, "");
    }

    #[test]
    fn test_decrypt_invalid_base64() {
        let result = decrypt("!!!not-base64!!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_data_too_short() {
        // 8 bytes < 12 bytes (nonce size)
        let short = BASE64.encode(&[0u8; 8]);
        let result = decrypt(&short);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_wrong_ciphertext() {
        // 12 bytes nonce + 1 byte garbage
        let data = vec![0u8; 13];
        let encoded = BASE64.encode(&data);
        let result = decrypt(&encoded);
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_produces_different_ciphertexts() {
        let plaintext = "same_input";
        let e1 = encrypt(plaintext).unwrap();
        let e2 = encrypt(plaintext).unwrap();
        // 不同 nonce 应产生不同密文
        assert_ne!(e1, e2);
        // 但都能正确解密
        assert_eq!(decrypt(&e1).unwrap(), plaintext);
        assert_eq!(decrypt(&e2).unwrap(), plaintext);
    }

    #[test]
    fn test_encrypt_unicode() {
        let plaintext = "你好世界🌍";
        let encrypted = encrypt(plaintext).unwrap();
        let decrypted = decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_long_string() {
        let plaintext = "a".repeat(10_000);
        let encrypted = encrypt(&plaintext).unwrap();
        let decrypted = decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
