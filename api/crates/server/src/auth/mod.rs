//! Authentication and Authorization Module

pub mod extractor;
pub mod handlers;
pub mod jwt;

pub use extractor::AuthenticatedUser;
pub use handlers::{login, me, register, AuthResponse, LoginRequest, RegisterRequest, UserDto};
pub use jwt::{decode_jwt, encode_jwt, hash_password, verify_password, Claims};
