//! Cryptographic services and zero-knowledge primitives.
//!
//! Provides memory-safe key derivation, authenticated symmetric encryption (AES-256-GCM),
//! and secure token generation.

pub mod cipher;
pub mod kdf;
pub mod memory;

pub use cipher::{AesGcmCipher, CipherError, EncryptedEnvelope};
pub use kdf::{DerivedKey, KdfError, KdfService};
pub use memory::{SecretBytes, SecretString};
