//! Authenticated Symmetric Encryption (AES-256-GCM)
//!
//! Provides authenticated encryption and decryption for sensitive user payloads
//! (e.g., personal journal entries and reflection notes) using AES-256-GCM with
//! cryptographically secure 96-bit random nonces.

use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Key, Nonce,
};
use rand_core::RngCore;

/// Size of the AES-256-GCM nonce in bytes (96 bits).
pub const NONCE_LENGTH: usize = 12;

/// Size of the AES-256-GCM key in bytes (256 bits).
pub const KEY_LENGTH: usize = 32;

/// Size of the AES-256-GCM authentication tag in bytes (128 bits).
pub const TAG_LENGTH: usize = 16;

/// Encrypted envelope containing the 96-bit random nonce and ciphertext with authentication tag.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EncryptedEnvelope {
    /// Cryptographically secure 96-bit random nonce used for this encryption operation.
    pub nonce: [u8; NONCE_LENGTH],

    /// Ciphertext payload with the 16-byte authentication tag appended.
    pub ciphertext: Vec<u8>,
}

/// Errors occurring during symmetric encryption or decryption operations.
#[derive(Debug, thiserror::Error)]
pub enum CipherError {
    /// Encryption failed due to an unexpected cryptographic error.
    #[error("Encryption failure: {0}")]
    EncryptionFailed(String),

    /// Decryption failed due to an invalid key, corrupted data, or tampered ciphertext.
    #[error("Decryption failed: invalid key or tampered ciphertext")]
    DecryptionFailed,
}

/// Service providing AES-256-GCM authenticated encryption and decryption.
#[derive(Debug, Clone, Copy, Default)]
pub struct AesGcmCipher;

impl AesGcmCipher {
    /// Encrypts `plaintext` using AES-256-GCM with the provided 32-byte `key` and optional `associated_data`.
    ///
    /// Generates a fresh 96-bit random nonce using [`rand_core::OsRng`] for every invocation.
    ///
    /// # Errors
    /// Returns [`CipherError::EncryptionFailed`] if encryption fails.
    pub fn encrypt(
        key: &[u8; KEY_LENGTH],
        plaintext: &[u8],
        associated_data: Option<&[u8]>,
    ) -> Result<EncryptedEnvelope, CipherError> {
        let mut nonce_bytes = [0u8; NONCE_LENGTH];
        rand_core::OsRng.fill_bytes(&mut nonce_bytes);

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        let nonce = Nonce::from_slice(&nonce_bytes);

        let payload = Payload {
            msg: plaintext,
            aad: associated_data.unwrap_or_default(),
        };

        let ciphertext = cipher
            .encrypt(nonce, payload)
            .map_err(|e| CipherError::EncryptionFailed(e.to_string()))?;

        Ok(EncryptedEnvelope {
            nonce: nonce_bytes,
            ciphertext,
        })
    }

    /// Decrypts an [`EncryptedEnvelope`] using AES-256-GCM with the provided 32-byte `key` and optional `associated_data`.
    ///
    /// Verifies the 128-bit authentication tag in constant time.
    ///
    /// # Errors
    /// Returns [`CipherError::DecryptionFailed`] if the key is incorrect, the ciphertext was tampered with,
    /// or associated data does not match. Details are intentionally opaque to prevent oracle attacks.
    pub fn decrypt(
        key: &[u8; KEY_LENGTH],
        envelope: &EncryptedEnvelope,
        associated_data: Option<&[u8]>,
    ) -> Result<Vec<u8>, CipherError> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        let nonce = Nonce::from_slice(&envelope.nonce);

        let payload = Payload {
            msg: &envelope.ciphertext,
            aad: associated_data.unwrap_or_default(),
        };

        cipher
            .decrypt(nonce, payload)
            .map_err(|_| CipherError::DecryptionFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_roundtrip() -> Result<(), CipherError> {
        let key = [7u8; KEY_LENGTH];
        let plaintext = b"Personal journal reflection entry.";

        let envelope = AesGcmCipher::encrypt(&key, plaintext, None)?;
        let decrypted = AesGcmCipher::decrypt(&key, &envelope, None)?;

        assert_eq!(decrypted, plaintext);
        Ok(())
    }

    #[test]
    fn test_encryption_roundtrip_with_associated_data() -> Result<(), CipherError> {
        let key = [42u8; KEY_LENGTH];
        let plaintext = b"Confidential notes with context.";
        let aad = b"user_id:12345|timestamp:2026-10-29";

        let envelope = AesGcmCipher::encrypt(&key, plaintext, Some(aad))?;
        let decrypted = AesGcmCipher::decrypt(&key, &envelope, Some(aad))?;

        assert_eq!(decrypted, plaintext);
        Ok(())
    }

    #[test]
    fn test_mismatched_associated_data_fails_decryption() -> Result<(), CipherError> {
        let key = [42u8; KEY_LENGTH];
        let plaintext = b"Confidential entry.";
        let aad_orig = b"context:v1";
        let aad_tampered = b"context:v2";

        let envelope = AesGcmCipher::encrypt(&key, plaintext, Some(aad_orig))?;
        let result = AesGcmCipher::decrypt(&key, &envelope, Some(aad_tampered));

        assert!(matches!(result, Err(CipherError::DecryptionFailed)));
        Ok(())
    }

    #[test]
    fn test_tampered_ciphertext_fails() -> Result<(), CipherError> {
        let key = [7u8; KEY_LENGTH];
        let plaintext = b"Confidential entry.";

        let mut envelope = AesGcmCipher::encrypt(&key, plaintext, None)?;
        if let Some(byte) = envelope.ciphertext.first_mut() {
            *byte ^= 0xFF;
        }

        let result = AesGcmCipher::decrypt(&key, &envelope, None);
        assert!(matches!(result, Err(CipherError::DecryptionFailed)));
        Ok(())
    }

    #[test]
    fn test_tampered_nonce_fails() -> Result<(), CipherError> {
        let key = [7u8; KEY_LENGTH];
        let plaintext = b"Entry with tampered nonce.";

        let mut envelope = AesGcmCipher::encrypt(&key, plaintext, None)?;
        envelope.nonce[0] ^= 0xFF;

        let result = AesGcmCipher::decrypt(&key, &envelope, None);
        assert!(matches!(result, Err(CipherError::DecryptionFailed)));
        Ok(())
    }

    #[test]
    fn test_wrong_key_fails_decryption() -> Result<(), CipherError> {
        let correct_key = [1u8; KEY_LENGTH];
        let wrong_key = [2u8; KEY_LENGTH];
        let plaintext = b"Payload for correct key.";

        let envelope = AesGcmCipher::encrypt(&correct_key, plaintext, None)?;
        let result = AesGcmCipher::decrypt(&wrong_key, &envelope, None);

        assert!(matches!(result, Err(CipherError::DecryptionFailed)));
        Ok(())
    }

    #[test]
    fn test_subsequent_encryptions_produce_unique_nonces() -> Result<(), CipherError> {
        let key = [9u8; KEY_LENGTH];
        let plaintext = b"Identical plaintext content.";

        let envelope1 = AesGcmCipher::encrypt(&key, plaintext, None)?;
        let envelope2 = AesGcmCipher::encrypt(&key, plaintext, None)?;

        assert_ne!(envelope1.nonce, envelope2.nonce);
        assert_ne!(envelope1.ciphertext, envelope2.ciphertext);
        Ok(())
    }

    #[test]
    fn test_empty_plaintext_roundtrip() -> Result<(), CipherError> {
        let key = [13u8; KEY_LENGTH];
        let plaintext = b"";

        let envelope = AesGcmCipher::encrypt(&key, plaintext, None)?;
        let decrypted = AesGcmCipher::decrypt(&key, &envelope, None)?;

        assert_eq!(decrypted, plaintext);
        assert_eq!(envelope.ciphertext.len(), TAG_LENGTH);
        Ok(())
    }
}
