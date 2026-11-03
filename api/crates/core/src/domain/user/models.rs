//! User Domain Models (RF05)

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// System user entity for authentication and multi-tenancy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub created_at: String,
}

impl User {
    /// Creates a new User instance.
    #[must_use]
    pub const fn new(id: Uuid, email: String, password_hash: String, created_at: String) -> Self {
        Self {
            id,
            email,
            password_hash,
            created_at,
        }
    }
}
