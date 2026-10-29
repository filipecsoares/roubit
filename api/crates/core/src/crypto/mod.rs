//! Cryptographic services and zero-knowledge primitives.
//!
//! Provides memory-safe key derivation, authenticated symmetric encryption (AES-256-GCM),
//! and secure token generation.

pub mod kdf;

pub use kdf::{DerivedKey, KdfError, KdfService};
