//! Structured telemetry and logging initialization.

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

/// Initializes structured logging with environment filter and compact formatting.
///
/// If a global tracing subscriber was already initialized (e.g. in test suites),
/// this function safely ignores the duplicate initialization without panicking.
pub fn init_telemetry(default_filter: &str) {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .compact();

    let _ = Registry::default()
        .with(env_filter)
        .with(fmt_layer)
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_idempotent_init() {
        init_telemetry("roubit=debug");
        init_telemetry("roubit=debug"); // Multiple calls must not panic
    }
}
