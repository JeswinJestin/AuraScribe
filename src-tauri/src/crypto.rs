// src-tauri/src/crypto.rs
//! Encryption utilities using AES-256-GCM with PBKDF2 key derivation

use anyhow::{Context, Result};
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use pbkdf2::pbkdf2_hmac_array;
use sha2::Sha256;
use zeroize::Zeroize;

const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;
const ITERATIONS: u32 = 256_000;

/// Encrypt plaintext with password using AES-256-GCM
pub fn encrypt(plaintext: &str, password: &str) -> Result<String> {
    // Generate random salt
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill(&mut salt);

    // Derive key from password
    let mut key = [0u8; KEY_LEN];
    pbkdf2_hmac_array::<Sha256, KEY_LEN>(password.as_bytes(), &salt, ITERATIONS, &mut key);

    // Create cipher
    let cipher = Aes256Gcm::new_from_slice(&key).context("Invalid key length")?;

    // Generate random nonce
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    // Encrypt
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .context("Encryption failed")?;

    // Combine: salt + nonce + ciphertext
    let mut combined = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    combined.extend_from_slice(&salt);
    combined.extend_from_slice(&nonce);
    combined.extend_from_slice(&ciphertext);

    // Zeroize key
    key.zeroize();

    Ok(base64::encode(combined))
}

/// Decrypt ciphertext with password
pub fn decrypt(ciphertext_b64: &str, password: &str) -> Result<String> {
    let combined = base64::decode(ciphertext_b64).context("Invalid base64")?;

    if combined.len() < SALT_LEN + NONCE_LEN {
        anyhow::bail!("Ciphertext too short");
    }

    // Extract components
    let salt = &combined[..SALT_LEN];
    let nonce_bytes = &combined[SALT_LEN..SALT_LEN + NONCE_LEN];
    let ciphertext = &combined[SALT_LEN + NONCE_LEN..];

    // Derive key
    let mut key = [0u8; KEY_LEN];
    pbkdf2_hmac_array::<Sha256, KEY_LEN>(password.as_bytes(), salt, ITERATIONS, &mut key);

    // Create cipher
    let cipher = Aes256Gcm::new_from_slice(&key).context("Invalid key length")?;

    // Decrypt
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .context("Decryption failed - wrong password or corrupted data")?;

    // Zeroize key
    key.zeroize();

    String::from_utf8(plaintext).context("Invalid UTF-8 in decrypted data")
}

/// Generate a secure random master key
pub fn generate_master_key() -> String {
    let mut key = [0u8; KEY_LEN];
    OsRng.fill(&mut key);
    base64::encode(key)
}

/// Hash a string for verification (not reversible)
pub fn hash_string(data: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    base64::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let password = "test-password-123";
        let plaintext = "Hello, World! This is a secret message.";

        let encrypted = encrypt(plaintext, password).unwrap();
        let decrypted = decrypt(&encrypted, password).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_wrong_password_fails() {
        let encrypted = encrypt("secret", "password1").unwrap();
        assert!(decrypt(&encrypted, "password2").is_err());
    }

    #[test]
    fn test_generate_master_key() {
        let key1 = generate_master_key();
        let key2 = generate_master_key();
        assert_ne!(key1, key2);
        assert!(!key1.is_empty());
    }
}