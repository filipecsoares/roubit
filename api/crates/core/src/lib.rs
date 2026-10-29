//! Roubit Core
//! Domain models, business logic, storage engines, and cryptographic services.

pub mod config;
pub mod telemetry;

pub use config::AppConfig;
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
