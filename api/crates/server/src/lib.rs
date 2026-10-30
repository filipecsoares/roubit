//! Roubit REST API Server Library

pub mod errors;
pub mod server;
pub mod state;

pub use errors::{ApiError, ProblemDetails};
pub use server::{
    build_router, fallback_handler, health_check, run_server, shutdown_signal, HealthResponse,
    REQUEST_ID_HEADER,
};
pub use state::AppState;
