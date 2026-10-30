//! Roubit CLI library module.
//!
//! Exposes argument parsers, command handlers, and terminal UI visualization.

pub mod args;
pub mod commands;
pub mod interactive;
pub mod ui;

pub use roubit_core::domain;
