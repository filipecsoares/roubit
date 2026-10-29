//! Goal & OKR Alignment Domain Module (RF03)
//!
//! Provides models, persistence, and habit linkage engines for Objectives and Key Results.

pub mod models;
pub mod repository;

pub use models::{KeyResult, Objective};
pub use repository::{
    ObjectiveWithKeyResults, OkrRepository, OkrRepositoryError, SqliteOkrRepository,
};
