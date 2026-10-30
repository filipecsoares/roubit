//! Terminal UI, formatting, tables, semantic colors, and progress indicators.

use std::io::IsTerminal;

use colored::Colorize;
use comfy_table::{
    modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Attribute, Cell, Color, ContentArrangement,
    Table,
};
use indicatif::{ProgressBar, ProgressStyle};
use roubit_core::domain::{routine::models::TimeBlock, BlockStatus};

/// Terminal user interface renderer for formatted output.
#[derive(Debug, Default)]
pub struct TerminalUi;

impl TerminalUi {
    /// Initializes terminal settings, disabling ANSI colors if standard output is redirected.
    pub fn init() {
        if !Self::is_stdout_terminal() {
            colored::control::set_override(false);
        }
    }

    /// Checks if standard output is attached to an interactive terminal (TTY).
    #[must_use]
    pub fn is_stdout_terminal() -> bool {
        std::io::stdout().is_terminal()
    }

    /// Formats a list of routine time blocks as an ASCII table string with rounded borders.
    #[must_use]
    pub fn format_routine_table(blocks: &[TimeBlock]) -> String {
        let now = chrono::Local::now().time();
        let is_tty = Self::is_stdout_terminal();

        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.apply_modifier(UTF8_ROUND_CORNERS);
        table.set_content_arrangement(ContentArrangement::Dynamic);
        table.set_header(vec!["Start", "End", "Category", "Name", "Status"]);

        for block in blocks {
            let start_str = block.start_time.format("%H:%M").to_string();
            let end_str = block.end_time.format("%H:%M").to_string();
            let is_active = now >= block.start_time && now < block.end_time;
            let is_past = now >= block.end_time;

            if is_active && is_tty {
                table.add_row(vec![
                    Cell::new(&start_str)
                        .fg(Color::Cyan)
                        .add_attribute(Attribute::Bold),
                    Cell::new(&end_str)
                        .fg(Color::Cyan)
                        .add_attribute(Attribute::Bold),
                    Cell::new(&block.category)
                        .fg(Color::Cyan)
                        .add_attribute(Attribute::Bold),
                    Cell::new(&block.name)
                        .fg(Color::Cyan)
                        .add_attribute(Attribute::Bold),
                    Cell::new("Active")
                        .fg(Color::Cyan)
                        .add_attribute(Attribute::Bold),
                ]);
            } else {
                let status_cell = if is_past {
                    if is_tty {
                        Cell::new("Done").fg(Color::Green)
                    } else {
                        Cell::new("Done")
                    }
                } else if is_active {
                    Cell::new("Active")
                } else if is_tty {
                    Cell::new("Pending").fg(Color::DarkGrey)
                } else {
                    Cell::new("Pending")
                };

                table.add_row(vec![
                    Cell::new(&start_str),
                    Cell::new(&end_str),
                    Cell::new(&block.category),
                    Cell::new(&block.name),
                    status_cell,
                ]);
            }
        }

        table.to_string()
    }

    /// Renders a list of routine time blocks directly to standard output.
    pub fn render_routine_table(blocks: &[TimeBlock]) {
        println!("{}", Self::format_routine_table(blocks));
    }

    /// Formats routine blocks along with their actual logged execution status.
    #[must_use]
    pub fn format_routine_table_with_statuses(
        blocks_with_status: &[(TimeBlock, BlockStatus)],
    ) -> String {
        let now = chrono::Local::now().time();
        let is_tty = Self::is_stdout_terminal();

        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.apply_modifier(UTF8_ROUND_CORNERS);
        table.set_content_arrangement(ContentArrangement::Dynamic);
        table.set_header(vec![
            "Start", "End", "Category", "Name", "Status", "Block ID",
        ]);

        for (block, status) in blocks_with_status {
            let start_str = block.start_time.format("%H:%M").to_string();
            let end_str = block.end_time.format("%H:%M").to_string();
            let is_active = now >= block.start_time && now < block.end_time;
            let id_short = block.id.to_string();

            let status_label = match status {
                BlockStatus::Completed => "Completed",
                BlockStatus::InProgress => "In Progress",
                BlockStatus::Skipped => "Skipped",
                BlockStatus::Pending => "Pending",
            };
            let (status_cell, is_highlighted) = match status {
                BlockStatus::Completed => (
                    if is_tty {
                        Cell::new(format!("✓ {status_label}")).fg(Color::Green)
                    } else {
                        Cell::new(format!("✓ {status_label}"))
                    },
                    false,
                ),
                BlockStatus::InProgress => (
                    if is_tty {
                        Cell::new(format!("▶ {status_label}"))
                            .fg(Color::Yellow)
                            .add_attribute(Attribute::Bold)
                    } else {
                        Cell::new(format!("▶ {status_label}"))
                    },
                    true,
                ),
                BlockStatus::Skipped => (
                    if is_tty {
                        Cell::new(format!("⊘ {status_label}")).fg(Color::DarkGrey)
                    } else {
                        Cell::new(format!("⊘ {status_label}"))
                    },
                    false,
                ),
                BlockStatus::Pending => {
                    if is_active && is_tty {
                        (
                            Cell::new(format!("● {status_label} (Active)"))
                                .fg(Color::Cyan)
                                .add_attribute(Attribute::Bold),
                            true,
                        )
                    } else if is_tty {
                        (Cell::new(status_label).fg(Color::DarkGrey), false)
                    } else {
                        (Cell::new(status_label), false)
                    }
                }
            };

            if is_highlighted && is_tty {
                table.add_row(vec![
                    Cell::new(&start_str)
                        .fg(Color::Cyan)
                        .add_attribute(Attribute::Bold),
                    Cell::new(&end_str)
                        .fg(Color::Cyan)
                        .add_attribute(Attribute::Bold),
                    Cell::new(&block.category)
                        .fg(Color::Cyan)
                        .add_attribute(Attribute::Bold),
                    Cell::new(&block.name)
                        .fg(Color::Cyan)
                        .add_attribute(Attribute::Bold),
                    status_cell,
                    Cell::new(&id_short)
                        .fg(Color::Cyan)
                        .add_attribute(Attribute::Bold),
                ]);
            } else {
                table.add_row(vec![
                    Cell::new(&start_str),
                    Cell::new(&end_str),
                    Cell::new(&block.category),
                    Cell::new(&block.name),
                    status_cell,
                    Cell::new(&id_short),
                ]);
            }
        }

        table.to_string()
    }

    /// Renders routine blocks with explicit logged statuses to standard output.
    pub fn render_routine_table_with_statuses(blocks_with_status: &[(TimeBlock, BlockStatus)]) {
        println!(
            "{}",
            Self::format_routine_table_with_statuses(blocks_with_status)
        );
    }

    /// Formats a list of habits and streak metrics into a table string.
    ///
    /// Each entry tuple contains: `(habit_name, target_description, streak_days, completed_today)`.
    #[must_use]
    pub fn format_habits_table(habits_with_streaks: &[(String, String, u32, bool)]) -> String {
        let is_tty = Self::is_stdout_terminal();
        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.apply_modifier(UTF8_ROUND_CORNERS);
        table.set_content_arrangement(ContentArrangement::Dynamic);
        table.set_header(vec![
            "Habit Name",
            "Target",
            "Streak (Days)",
            "Today's Status",
        ]);

        for (name, target, streak, completed_today) in habits_with_streaks {
            let streak_str = if *streak > 0 {
                format!("🔥 {streak}d")
            } else {
                "0d".to_string()
            };

            let streak_cell = if *streak > 0 && is_tty {
                Cell::new(&streak_str).fg(Color::Yellow)
            } else {
                Cell::new(&streak_str)
            };

            let status_cell = if *completed_today {
                if is_tty {
                    Cell::new("✓ Completed").fg(Color::Green)
                } else {
                    Cell::new("✓ Completed")
                }
            } else if is_tty {
                Cell::new("○ Pending").fg(Color::Yellow)
            } else {
                Cell::new("○ Pending")
            };

            let name_cell = if is_tty {
                Cell::new(name).add_attribute(Attribute::Bold)
            } else {
                Cell::new(name)
            };

            table.add_row(vec![name_cell, Cell::new(target), streak_cell, status_cell]);
        }

        table.to_string()
    }

    /// Renders the habits table directly to standard output.
    pub fn render_habits_table(habits_with_streaks: &[(String, String, u32, bool)]) {
        println!("{}", Self::format_habits_table(habits_with_streaks));
    }

    /// Formats an OKR progress bar string e.g. `Title [████████░░░░] 65.0%`.
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn format_okr_progress(title: &str, progress: f32) -> String {
        const TOTAL_WIDTH: usize = 20;
        const TOTAL_WIDTH_F32: f32 = 20.0;

        let is_tty = Self::is_stdout_terminal();
        let pct = if progress <= 1.0 && progress > 0.0 {
            progress * 100.0
        } else {
            progress
        }
        .clamp(0.0, 100.0);

        let filled_count = ((pct / 100.0) * TOTAL_WIDTH_F32).round() as usize;
        let filled = filled_count.min(TOTAL_WIDTH);
        let empty = TOTAL_WIDTH.saturating_sub(filled);

        let filled_bar = "█".repeat(filled);
        let empty_bar = "░".repeat(empty);

        let bar_rendered = if is_tty {
            if pct >= 100.0 {
                format!("{}{}", filled_bar.green(), empty_bar.dimmed())
            } else if pct >= 50.0 {
                format!("{}{}", filled_bar.cyan(), empty_bar.dimmed())
            } else if pct > 0.0 {
                format!("{}{}", filled_bar.yellow(), empty_bar.dimmed())
            } else {
                format!("{}{}", filled_bar, empty_bar.dimmed())
            }
        } else {
            format!("{filled_bar}{empty_bar}")
        };

        format!("{title:<35} [{bar_rendered}] {pct:>5.1}%")
    }

    /// Renders an OKR progress bar directly to standard output.
    pub fn render_okr_progress(title: &str, progress: f32) {
        println!("{}", Self::format_okr_progress(title, progress));
    }

    /// Formats a success message with green checkmark.
    #[must_use]
    pub fn format_success(msg: &str) -> String {
        if Self::is_stdout_terminal() {
            format!("{} {}", "✓".green().bold(), msg)
        } else {
            format!("[SUCCESS] {msg}")
        }
    }

    /// Prints a success message to standard output.
    pub fn print_success(msg: &str) {
        println!("{}", Self::format_success(msg));
    }

    /// Formats a warning message with yellow alert icon.
    #[must_use]
    pub fn format_warning(msg: &str) -> String {
        if Self::is_stdout_terminal() {
            format!("{} {}", "⚠".yellow().bold(), msg)
        } else {
            format!("[WARN] {msg}")
        }
    }

    /// Prints a warning message to standard output.
    pub fn print_warning(msg: &str) {
        println!("{}", Self::format_warning(msg));
    }

    /// Formats an error message with red cross mark.
    #[must_use]
    pub fn format_error(msg: &str) -> String {
        if Self::is_stdout_terminal() {
            format!("{} {}", "✖".red().bold(), msg)
        } else {
            format!("[ERROR] {msg}")
        }
    }

    /// Prints an error message to standard error.
    pub fn print_error(msg: &str) {
        eprintln!("{}", Self::format_error(msg));
    }

    /// Creates an interactive terminal spinner for longer-running actions.
    #[must_use]
    pub fn create_spinner(message: &str) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        if let Ok(style) = ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
            .template("{spinner:.green} {msg}")
        {
            pb.set_style(style);
        }
        pb.set_message(message.to_string());
        pb
    }

    /// Creates a bounded progress bar for multi-step tasks.
    #[must_use]
    pub fn create_progress_bar(total: u64, message: &str) -> ProgressBar {
        let pb = ProgressBar::new(total);
        if let Ok(style) = ProgressStyle::default_bar()
            .template("{prefix:<20} [{bar:25.cyan/blue}] {pos}/{len} ({percent}%)")
        {
            pb.set_style(style.progress_chars("█░ "));
        }
        pb.set_prefix(message.to_string());
        pb
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::{NaiveTime, Utc};
    use uuid::Uuid;

    #[test]
    fn test_table_formatting_does_not_panic() {
        let mut table = comfy_table::Table::new();
        table.set_header(vec!["Col 1", "Col 2"]);
        table.add_row(vec!["Val 1", "Val 2"]);
        let output = table.to_string();
        assert!(output.contains("Val 1"));
        assert!(output.contains("Val 2"));
    }

    #[test]
    fn test_routine_table_renders_blocks() {
        let block = TimeBlock {
            id: Uuid::new_v4(),
            routine_id: Uuid::new_v4(),
            name: "Morning Focus".to_string(),
            start_time: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            end_time: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            category: "Deep Work".to_string(),
            color_hex: "#6366F1".to_string(),
            order_index: 0,
            created_at: Utc::now(),
        };

        let output = TerminalUi::format_routine_table(&[block]);
        assert!(output.contains("Morning Focus"));
        assert!(output.contains("Deep Work"));
        assert!(output.contains("08:00"));
        assert!(output.contains("10:00"));
    }

    #[test]
    fn test_routine_table_with_statuses_renders() {
        let block = TimeBlock {
            id: Uuid::new_v4(),
            routine_id: Uuid::new_v4(),
            name: "Exercise".to_string(),
            start_time: NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
            end_time: NaiveTime::from_hms_opt(19, 0, 0).unwrap(),
            category: "Health".to_string(),
            color_hex: "#10B981".to_string(),
            order_index: 1,
            created_at: Utc::now(),
        };

        let output =
            TerminalUi::format_routine_table_with_statuses(&[(block, BlockStatus::Completed)]);
        assert!(output.contains("Exercise"));
        assert!(output.contains("Health"));
        assert!(output.contains("Completed"));
    }

    #[test]
    fn test_habits_table_formatting() {
        let data = vec![
            ("Drink Water".to_string(), "2000 ml".to_string(), 7, true),
            ("Read Book".to_string(), "30 mins".to_string(), 0, false),
        ];

        let output = TerminalUi::format_habits_table(&data);
        assert!(output.contains("Drink Water"));
        assert!(output.contains("2000 ml"));
        assert!(output.contains("7d"));
        assert!(output.contains("Completed"));
        assert!(output.contains("Read Book"));
        assert!(output.contains("Pending"));
    }

    #[test]
    fn test_okr_progress_formatting() {
        let output = TerminalUi::format_okr_progress("Ship Roubit v1.0", 65.0);
        assert!(output.contains("Ship Roubit v1.0"));
        assert!(output.contains("65.0%"));
        assert!(output.contains('█'));
        assert!(output.contains('░'));

        // Test normalized ratio <= 1.0
        let output_norm = TerminalUi::format_okr_progress("Read Books", 0.5);
        assert!(output_norm.contains("50.0%"));
    }

    #[test]
    fn test_status_message_formatting() {
        let succ = TerminalUi::format_success("Task finished");
        assert!(succ.contains("Task finished"));

        let warn = TerminalUi::format_warning("Approaching deadline");
        assert!(warn.contains("Approaching deadline"));

        let err = TerminalUi::format_error("Failed to connect");
        assert!(err.contains("Failed to connect"));
    }

    #[test]
    fn test_progress_bar_and_spinner_instantiation() {
        let spinner = TerminalUi::create_spinner("Working...");
        spinner.finish();

        let pb = TerminalUi::create_progress_bar(100, "Processing items");
        pb.set_position(50);
        pb.finish();
    }
}
