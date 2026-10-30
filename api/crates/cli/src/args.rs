//! Command-line argument definitions and Clap schema.

use clap::{Args, Parser, Subcommand};

/// High-performance personal platform for routines, habits, OKRs, and encrypted journaling.
#[derive(Parser, Debug, Clone, PartialEq)]
#[command(
    name = "roubit",
    author,
    version,
    about = "Personal high-performance routine, habit, OKR, and encrypted journaling platform",
    long_about = None
)]
pub struct CliArgs {
    /// Path to local SQLite database file or database connection URL.
    #[arg(
        global = true,
        short = 'd',
        long,
        help = "Path to database file or SQLite URL"
    )]
    pub db: Option<String>,

    /// Remote HTTP API base URL (e.g., <https://api.roubit.local>).
    #[arg(
        global = true,
        short = 'r',
        long,
        env = "ROUBIT_REMOTE_URL",
        help = "Remote API base URL"
    )]
    pub remote: Option<String>,

    /// Output responses in machine-readable raw JSON format.
    #[arg(global = true, long, help = "Output response in raw JSON format")]
    pub json: bool,

    /// Increase logging and execution verbosity.
    #[arg(
        global = true,
        short = 'v',
        long,
        action = clap::ArgAction::Count,
        help = "Increase logging verbosity"
    )]
    pub verbose: u8,

    /// Available CLI subcommands.
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Core subcommand categories supported by Roubit.
#[derive(Subcommand, Debug, Clone, PartialEq)]
pub enum Commands {
    /// Routine schedule management and daily time-blocking (RF01).
    Routine(RoutineArgs),

    /// Habit tracking, daily check-ins, and streak calculation (RF02).
    Habit(HabitArgs),

    /// Goal alignment, quarterly objectives, and key result tracking (RF03).
    Okr(OkrArgs),

    /// Zero-knowledge encrypted reflection journaling (RF04).
    Journal(JournalArgs),

    /// Local settings and storage configuration.
    Config(ConfigArgs),

    /// Administrative utilities (database migrations).
    Admin(AdminArgs),
}

// ---------------------------------------------------------------------------
// 1. Routine Subcommands
// ---------------------------------------------------------------------------

/// Arguments for routine management.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct RoutineArgs {
    /// Action to execute on routines.
    #[command(subcommand)]
    pub action: RoutineSubcommands,
}

/// Supported routine actions.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum RoutineSubcommands {
    /// List time blocks and execution adherence for a given date.
    List {
        /// Date in YYYY-MM-DD format (defaults to current date).
        #[arg(long, help = "Date in YYYY-MM-DD format (defaults to today)")]
        date: Option<String>,
    },

    /// Mark a scheduled time block as in-progress.
    Start {
        /// Unique ID of the time block to start.
        block_id: String,

        /// Target date in YYYY-MM-DD format (defaults to current date).
        #[arg(long, help = "Target date in YYYY-MM-DD format")]
        date: Option<String>,
    },

    /// Mark a scheduled time block as completed.
    Complete {
        /// Unique ID of the time block to complete.
        block_id: String,

        /// Target date in YYYY-MM-DD format (defaults to current date).
        #[arg(long, help = "Target date in YYYY-MM-DD format")]
        date: Option<String>,
    },

    /// Mark a scheduled time block as skipped.
    Skip {
        /// Unique ID of the time block to skip.
        block_id: String,

        /// Target date in YYYY-MM-DD format (defaults to current date).
        #[arg(long, help = "Target date in YYYY-MM-DD format")]
        date: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// 2. Habit Subcommands
// ---------------------------------------------------------------------------

/// Arguments for habit tracking.
#[derive(Args, Debug, Clone, PartialEq)]
pub struct HabitArgs {
    /// Action to execute on habits.
    #[command(subcommand)]
    pub action: HabitSubcommands,
}

/// Supported habit actions.
#[derive(Subcommand, Debug, Clone, PartialEq)]
pub enum HabitSubcommands {
    /// List active or archived habits.
    List {
        /// Include archived habits in output.
        #[arg(short = 'a', long, help = "Show archived habits")]
        all: bool,
    },

    /// Record a check-in progress entry for a habit.
    Checkin {
        /// Unique identifier of the habit.
        habit_id: String,

        /// Value for numeric or timer habits (defaults to 1.0 for binary habits).
        #[arg(long, help = "Value for numeric or timer habits (default: 1.0)")]
        value: Option<f64>,

        /// Date in YYYY-MM-DD format (defaults to current date).
        #[arg(long, help = "Date in YYYY-MM-DD format")]
        date: Option<String>,
    },

    /// Display streak statistics and consistency metrics.
    Streaks,
}

// ---------------------------------------------------------------------------
// 3. OKR Subcommands
// ---------------------------------------------------------------------------

/// Arguments for OKR management.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct OkrArgs {
    /// Action to execute on OKRs.
    #[command(subcommand)]
    pub action: OkrSubcommands,
}

/// Supported OKR actions.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum OkrSubcommands {
    /// List quarterly objectives and their key results.
    List {
        /// Filter objectives by quarter (e.g. "2026-Q4").
        #[arg(short = 'q', long, help = "Filter by quarter (e.g., 2026-Q4)")]
        quarter: Option<String>,
    },

    /// Display a high-level summary of active objectives and progress roll-ups.
    Summary,
}

// ---------------------------------------------------------------------------
// 4. Journal Subcommands
// ---------------------------------------------------------------------------

/// Arguments for zero-knowledge journaling.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct JournalArgs {
    /// Action to execute on the journal.
    #[command(subcommand)]
    pub action: JournalSubcommands,
}

/// Supported journaling actions.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum JournalSubcommands {
    /// Write or update an encrypted journal entry.
    Write {
        /// Target calendar date in YYYY-MM-DD format (defaults to today).
        #[arg(long, help = "Date in YYYY-MM-DD format")]
        date: Option<String>,

        /// Self-reported mood rating from 1 (very bad) to 5 (excellent).
        #[arg(short = 'm', long, help = "Mood score from 1 to 5")]
        mood: Option<u8>,

        /// Comma-separated list of categorical tags.
        #[arg(
            short = 't',
            long,
            value_delimiter = ',',
            help = "Comma-separated tags"
        )]
        tags: Vec<String>,

        /// Explicit reflection content (if omitted, prompts interactively).
        #[arg(short = 'c', long, help = "Plaintext reflection content")]
        content: Option<String>,
    },

    /// Decrypt and read a reflection entry with master passphrase.
    Read {
        /// Target calendar date in YYYY-MM-DD format (defaults to today).
        #[arg(long, help = "Date in YYYY-MM-DD format")]
        date: Option<String>,

        /// Specific entry ID to read.
        #[arg(long, help = "Specific entry UUID")]
        id: Option<String>,
    },

    /// List open metadata entries across a date range without decrypting payloads.
    List {
        /// Inclusive start date in YYYY-MM-DD format.
        #[arg(long, help = "Start date in YYYY-MM-DD format")]
        from: Option<String>,

        /// Inclusive end date in YYYY-MM-DD format.
        #[arg(long, help = "End date in YYYY-MM-DD format")]
        to: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// 5. Config Subcommands
// ---------------------------------------------------------------------------

/// Arguments for CLI and environment configuration.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct ConfigArgs {
    /// Configuration action.
    #[command(subcommand)]
    pub action: ConfigSubcommands,
}

/// Supported configuration actions.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum ConfigSubcommands {
    /// Show effective configuration settings.
    Show,

    /// Update configuration settings.
    Set {
        /// Set local SQLite database path.
        #[arg(long, help = "Local database file path")]
        db: Option<String>,

        /// Set remote API base URL.
        #[arg(long, help = "Remote API base URL")]
        api_url: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// 6. Admin Subcommands
// ---------------------------------------------------------------------------

/// Arguments for administrative utilities.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct AdminArgs {
    /// Administrative action.
    #[command(subcommand)]
    pub action: AdminAction,
}

/// Supported administrative actions.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum AdminAction {
    /// Migrates an existing SQLite database to PostgreSQL.
    MigrateToPostgres {
        /// Path to source SQLite database file or URL.
        #[arg(long, help = "Path to source SQLite database")]
        from: String,

        /// Target PostgreSQL connection URL.
        #[arg(long, help = "Target PostgreSQL connection URL")]
        to: String,
    },
}
