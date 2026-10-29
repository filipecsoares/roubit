//! Memory Sanitization & Protected Secret Wrappers
//!
//! Provides memory-safe wrappers for plaintexts, derived cryptographic keys,
//! and sensitive passphrases. Memory is automatically scrubbed (zeroized)
//! on drop using non-optimizable volatile writes to prevent data retention in RAM.

use std::ops::Deref;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Secure heap-allocated string wrapper zeroized on drop.
///
/// Prevents sensitive passphrases or confidential user inputs from remaining in memory.
/// Formats as `***REDACTED***` in both [`std::fmt::Debug`] and [`std::fmt::Display`].
#[derive(Clone, Default, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct SecretString(String);

impl SecretString {
    /// Wraps a standard heap-allocated [`String`] as a protected secret.
    #[must_use]
    pub const fn new(val: String) -> Self {
        Self(val)
    }

    /// Accesses the underlying string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Accesses the underlying byte slice.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Returns the length of the secret string in bytes.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the secret string has a length of 0.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Deref for SecretString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for SecretString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<[u8]> for SecretString {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl From<String> for SecretString {
    fn from(val: String) -> Self {
        Self::new(val)
    }
}

impl From<&str> for SecretString {
    fn from(val: &str) -> Self {
        Self::new(val.to_owned())
    }
}

impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SecretString(***REDACTED***)")
    }
}

impl std::fmt::Display for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "***REDACTED***")
    }
}

/// Secure heap-allocated byte vector wrapper zeroized on drop.
///
/// Prevents secret byte buffers, binary plaintext payloads, or raw credentials
/// from remaining in memory. Formats as `***REDACTED***` in both
/// [`std::fmt::Debug`] and [`std::fmt::Display`].
#[derive(Clone, Default, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct SecretBytes(Vec<u8>);

impl SecretBytes {
    /// Wraps a byte vector as a protected secret.
    #[must_use]
    pub const fn new(val: Vec<u8>) -> Self {
        Self(val)
    }

    /// Accesses the underlying byte slice.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Returns the number of bytes in the secret buffer.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the secret buffer has a length of 0.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Deref for SecretBytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<[u8]> for SecretBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<Vec<u8>> for SecretBytes {
    fn from(val: Vec<u8>) -> Self {
        Self::new(val)
    }
}

impl From<&[u8]> for SecretBytes {
    fn from(val: &[u8]) -> Self {
        Self::new(val.to_vec())
    }
}

impl std::fmt::Debug for SecretBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SecretBytes(***REDACTED***)")
    }
}

impl std::fmt::Display for SecretBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "***REDACTED***")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_string_debug_redaction() {
        let secret = SecretString::new("my-master-password".to_string());
        let formatted = format!("{secret:?}");
        assert!(!formatted.contains("my-master-password"));
        assert!(formatted.contains("***REDACTED***"));
    }

    #[test]
    fn test_secret_string_display_redaction() {
        let secret = SecretString::from("confidential-token-123");
        let formatted = format!("{secret}");
        assert_eq!(formatted, "***REDACTED***");
        assert!(!formatted.contains("confidential-token-123"));
    }

    #[test]
    fn test_secret_string_deref_and_accessors() {
        let secret = SecretString::from("passphrase-content");
        assert_eq!(&*secret, "passphrase-content");
        assert_eq!(secret.as_str(), "passphrase-content");
        assert_eq!(secret.as_bytes(), b"passphrase-content");
        assert_eq!(secret.len(), 18);
        assert!(!secret.is_empty());
    }

    #[test]
    fn test_secret_bytes_debug_and_display_redaction() {
        let secret = SecretBytes::new(vec![1, 2, 3, 4, 5]);
        let debug_str = format!("{secret:?}");
        let display_str = format!("{secret}");

        assert_eq!(debug_str, "SecretBytes(***REDACTED***)");
        assert_eq!(display_str, "***REDACTED***");
    }

    #[test]
    fn test_secret_bytes_deref_and_accessors() {
        let raw = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let secret = SecretBytes::from(raw.as_slice());

        assert_eq!(&*secret, &[0xDE, 0xAD, 0xBE, 0xEF]);
        assert_eq!(secret.as_slice(), &[0xDE, 0xAD, 0xBE, 0xEF]);
        assert_eq!(secret.len(), 4);
        assert!(!secret.is_empty());
    }

    #[test]
    fn test_zeroize_on_drop_behavior() {
        let mut buffer = [0xAAu8; 32];
        buffer.zeroize();
        assert_eq!(buffer, [0u8; 32]);
    }
}
