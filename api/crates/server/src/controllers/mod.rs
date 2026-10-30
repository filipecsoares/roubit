//! Domain REST API Controllers

pub mod habit_controller;
pub mod journal_controller;
pub mod okr_controller;
pub mod routine_controller;

pub use habit_controller::{
    habit_routes, CheckinResponse, CreateHabitRequest, HabitCheckinRequest,
};
pub use journal_controller::{
    journal_routes, EncryptedJournalResponse, JournalMetadataQuery, SaveEncryptedJournalRequest,
};
pub use okr_controller::{
    okr_routes, CreateKeyResultRequest, CreateObjectiveRequest, ListOkrsQuery,
};
pub use routine_controller::{
    complete_block, create_routine, get_routine, routine_routes, CreateRoutineRequest,
    CreateTimeBlockRequest, LogBlockStatusRequest, RoutineWithProgressResponse,
};
