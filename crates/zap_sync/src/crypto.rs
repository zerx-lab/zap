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

/// 从 Token 派生 32 字节密钥
fn derive_key(token: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hasher.finalize().into()
}

/// 使用 AES-256-GCM 加密明文，返回 Base64 编码的 nonce+ciphertext
pub fn encrypt(token: &str, plaintext: &str) -> Result<String, CryptoError> {
    let key = derive_key(token);
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
pub fn decrypt(token: &str, encoded: &str) -> Result<String, CryptoError> {
    let key = derive_key(token);
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
        let token = "ghp_test_token_12345";
        let plaintext = "my_secret_password";
        let encrypted = encrypt(token, plaintext).unwrap();
        let decrypted = decrypt(token, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_different_tokens_produce_different_ciphertext() {
        let encrypted1 = encrypt("token_a", "secret").unwrap();
        let encrypted2 = encrypt("token_b", "secret").unwrap();
        assert_ne!(encrypted1, encrypted2);
    }

    #[test]
    fn test_wrong_token_fails_to_decrypt() {
        let encrypted = encrypt("correct_token", "secret").unwrap();
        assert!(decrypt("wrong_token", &encrypted).is_err());
    }

    #[test]
    fn test_empty_string() {
        let encrypted = encrypt("token", "").unwrap();
        let decrypted = decrypt("token", &encrypted).unwrap();
        assert_eq!(decrypted, "");
    }
}
