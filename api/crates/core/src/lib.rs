//! Roubit Core
//! Domain models, business logic, storage engines, and cryptographic services.

pub mod config;
pub mod crypto;
pub mod domain;
pub mod telemetry;

pub use config::AppConfig;
pub use crypto::{
    AesGcmCipher, CipherError, DerivedKey, EncryptedEnvelope, KdfError, KdfService, SecretBytes,
    SecretString,
};
pub use domain::{BlockStatus, DailyRoutineLog, Routine, TimeBlock};
pub use telemetry::init_telemetry;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_version() {
        assert_eq!(VERSION, "0.1.0");
    }
}
