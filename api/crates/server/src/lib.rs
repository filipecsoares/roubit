//! Roubit REST API Server Library

pub mod auth;
pub mod errors;
pub mod middleware;
pub mod server;
pub mod state;

pub use auth::{
    decode_jwt, encode_jwt, hash_password, verify_password, AuthResponse, AuthenticatedUser,
    Claims, LoginRequest, RegisterRequest, UserDto,
};
pub use errors::{ApiError, ProblemDetails};
pub use middleware::{
    build_cors_layer, idempotency_middleware, purge_expired_keys, security_headers_middleware,
    IDEMPOTENCY_KEY_HEADER, X_CACHE_HEADER,
};
pub use server::{
    build_router, fallback_handler, health_check, run_server, shutdown_signal, HealthResponse,
    REQUEST_ID_HEADER,
};
pub use state::AppState;
