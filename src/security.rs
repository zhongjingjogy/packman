use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, OsRng},
};
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString},
};
use base64::{Engine as _, engine::general_purpose};
use std::env;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("Environment variable BEEPKG_USER_SECRET not set")]
    MissingSecret,
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
    #[error("Password hashing failed: {0}")]
    HashingFailed(String),
}

pub struct SecurityManager;

impl SecurityManager {
    pub fn new() -> Self {
        Self
    }

    /// 从环境变量获取密码
    fn get_secret() -> Result<String, SecurityError> {
        env::var("BEEPKG_USER_SECRET").map_err(|_| SecurityError::MissingSecret)
    }

    /// 加密数据
    pub fn encrypt_data(data: &[u8]) -> Result<(String, String), SecurityError> {
        let password = Self::get_secret()?;

        // 生成随机盐值
        let salt = SaltString::generate(&mut OsRng);

        // 使用Argon2派生密钥
        let argon2 = Argon2::default();
        let key = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| SecurityError::HashingFailed(e.to_string()))?
            .hash
            .ok_or_else(|| SecurityError::HashingFailed("No hash generated".to_string()))?;

        let key = key.as_bytes();
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| SecurityError::EncryptionFailed(e.to_string()))?;

        // 生成随机nonce
        let nonce_bytes = rand::random::<[u8; 12]>();
        let nonce = Nonce::from_slice(&nonce_bytes);

        // 加密数据
        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|e| SecurityError::EncryptionFailed(e.to_string()))?;

        // 返回base64编码的加密数据和盐值
        Ok((
            general_purpose::STANDARD.encode(ciphertext),
            salt.to_string(),
        ))
    }

    /// 解密数据
    pub fn decrypt_data(encrypted: &str, salt: &str) -> Result<Vec<u8>, SecurityError> {
        let password = Self::get_secret()?;

        // 使用盐值派生密钥
        let argon2 = Argon2::default();
        let salt =
            SaltString::new(salt).map_err(|e| SecurityError::DecryptionFailed(e.to_string()))?;

        let key = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| SecurityError::DecryptionFailed(e.to_string()))?
            .hash
            .ok_or_else(|| SecurityError::DecryptionFailed("No hash generated".to_string()))?;

        let key = key.as_bytes();
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| SecurityError::DecryptionFailed(e.to_string()))?;

        // 解码base64数据
        let ciphertext = general_purpose::STANDARD
            .decode(encrypted)
            .map_err(|e| SecurityError::DecryptionFailed(e.to_string()))?;

        // 使用固定nonce (实际应用中应该存储nonce)
        let nonce = Nonce::from_slice(&[0; 12]);

        // 解密数据
        cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| SecurityError::DecryptionFailed(e.to_string()))
    }
}
