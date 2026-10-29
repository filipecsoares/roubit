//! Argon2id Key Derivation Service
//!
//! Provides cryptographically secure derivation of 256-bit encryption keys
//! from user passphrases using Argon2id with OWASP-recommended parameters.

use rand_core::RngCore;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Secure wrapper for derived 256-bit master key.
///
/// Implements [`ZeroizeOnDrop`] to ensure that secret key material is zeroed out
/// from memory when dropped. The [`std::fmt::Debug`] implementation redacts the
/// key bytes to prevent accidental leakage in logs.
#[derive(Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct DerivedKey(pub [u8; 32]);

impl DerivedKey {
    /// Accesses the underlying 32-byte key slice.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::ops::Deref for DerivedKey {
    type Target = [u8; 32];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<[u8; 32]> for DerivedKey {
    fn as_ref(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Debug for DerivedKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DerivedKey([REDACTED])")
    }
}

impl std::fmt::Display for DerivedKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("***REDACTED***")
    }
}

/// Errors occurring during key derivation operations.
#[derive(Debug, thiserror::Error)]
pub enum KdfError {
    /// Argon2 key derivation failure.
    #[error("Argon2 derivation error: {0}")]
    DerivationFailed(String),

    /// Salt length is less than the required 16 bytes.
    #[error("Invalid salt length, expected at least 16 bytes")]
    InvalidSalt,
}

/// Service for deriving cryptographic master keys using Argon2id.
#[derive(Debug, Clone)]
pub struct KdfService {
    memory_cost: u32,
    time_cost: u32,
    parallelism: u32,
}

impl KdfService {
    /// OWASP recommended memory cost: 64 MiB (65536 KiB).
    pub const OWASP_MEMORY_COST: u32 = 65536;

    /// OWASP recommended time cost (iterations): 3 passes.
    pub const OWASP_TIME_COST: u32 = 3;

    /// OWASP recommended parallelism: 4 lanes/threads.
    pub const OWASP_PARALLELISM: u32 = 4;

    /// Derived key length in bytes: 32 bytes (256 bits).
    pub const KEY_LENGTH: usize = 32;

    /// Minimum recommended salt length: 16 bytes (128 bits).
    pub const MIN_SALT_LENGTH: usize = 16;

    /// Creates a new `KdfService` with custom Argon2id parameters.
    #[must_use]
    pub const fn new(memory_cost: u32, time_cost: u32, parallelism: u32) -> Self {
        Self {
            memory_cost,
            time_cost,
            parallelism,
        }
    }

    /// Creates a new `KdfService` configured with OWASP recommended parameters:
    /// - Memory: 64 MiB (65,536 KiB)
    /// - Time: 3 iterations
    /// - Parallelism: 4 threads
    #[must_use]
    pub const fn default_recommended() -> Self {
        Self::new(
            Self::OWASP_MEMORY_COST,
            Self::OWASP_TIME_COST,
            Self::OWASP_PARALLELISM,
        )
    }

    /// Generates a cryptographically secure 16-byte random salt using `rand_core::OsRng`.
    #[must_use]
    pub fn generate_salt() -> [u8; 16] {
        let mut salt = [0u8; 16];
        rand_core::OsRng.fill_bytes(&mut salt);
        salt
    }

    /// Derives a 256-bit `DerivedKey` from a passphrase and salt using Argon2id.
    ///
    /// # Errors
    /// Returns [`KdfError::InvalidSalt`] if `salt` length is less than 16 bytes.
    /// Returns [`KdfError::DerivationFailed`] if parameter validation or hashing fails.
    pub fn derive_key(&self, passphrase: &[u8], salt: &[u8]) -> Result<DerivedKey, KdfError> {
        if salt.len() < Self::MIN_SALT_LENGTH {
            return Err(KdfError::InvalidSalt);
        }

        let params = argon2::Params::new(
            self.memory_cost,
            self.time_cost,
            self.parallelism,
            Some(Self::KEY_LENGTH),
        )
        .map_err(|e| KdfError::DerivationFailed(e.to_string()))?;

        let argon2 =
            argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

        let mut key = [0u8; Self::KEY_LENGTH];
        argon2
            .hash_password_into(passphrase, salt, &mut key)
            .map_err(|e| KdfError::DerivationFailed(e.to_string()))?;

        Ok(DerivedKey(key))
    }
}

impl Default for KdfService {
    fn default() -> Self {
        Self::default_recommended()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argon2id_derivation_deterministic() -> Result<(), KdfError> {
        let kdf = KdfService::default_recommended();
        let salt = [42u8; 16];
        let pass = b"correct-horse-battery-staple";

        let key1 = kdf.derive_key(pass, &salt)?;
        let key2 = kdf.derive_key(pass, &salt)?;

        assert_eq!(key1.0, key2.0);
        assert_eq!(key1, key2);
        Ok(())
    }

    #[test]
    fn test_argon2id_different_salts_produce_different_keys() -> Result<(), KdfError> {
        let kdf = KdfService::default_recommended();
        let pass = b"secure-passphrase";

        let key1 = kdf.derive_key(pass, &[1u8; 16])?;
        let key2 = kdf.derive_key(pass, &[2u8; 16])?;

        assert_ne!(key1.0, key2.0);
        assert_ne!(key1, key2);
        Ok(())
    }

    #[test]
    fn test_argon2id_different_passphrases_produce_different_keys() -> Result<(), KdfError> {
        let kdf = KdfService::default_recommended();
        let salt = [99u8; 16];

        let key1 = kdf.derive_key(b"passphrase-a", &salt)?;
        let key2 = kdf.derive_key(b"passphrase-b", &salt)?;

        assert_ne!(key1, key2);
        Ok(())
    }

    #[test]
    fn test_salt_validation_rejects_too_short() {
        let kdf = KdfService::default_recommended();
        let short_salt = [1u8; 15]; // < 16 bytes

        let result = kdf.derive_key(b"some-passphrase", &short_salt);
        assert!(matches!(result, Err(KdfError::InvalidSalt)));
    }

    #[test]
    fn test_generate_salt_randomness() {
        let salt1 = KdfService::generate_salt();
        let salt2 = KdfService::generate_salt();

        assert_eq!(salt1.len(), 16);
        assert_eq!(salt2.len(), 16);
        assert_ne!(
            salt1, salt2,
            "Subsequent salts generated via OsRng must differ"
        );
    }

    #[test]
    fn test_debug_redacts_key_material() {
        let key = DerivedKey([7u8; 32]);
        let debug_str = format!("{key:?}");
        assert_eq!(debug_str, "DerivedKey([REDACTED])");
        assert!(!debug_str.contains('7'));
    }

    #[test]
    fn test_derived_key_as_bytes() {
        let key = DerivedKey([5u8; 32]);
        assert_eq!(key.as_bytes(), &[5u8; 32]);
    }
}
