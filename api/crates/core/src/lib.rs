//! Roubit Core
//! Domain models, business logic, storage engines, and cryptographic services.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_version() {
        assert_eq!(VERSION, "0.1.0");
    }
}
