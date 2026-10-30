//! Interactive prompts, secure passphrase inputs, and external editor integration.

use std::io::{self, BufRead, Read, Seek, SeekFrom, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

use roubit_core::crypto::memory::SecretString;

/// Interactive prompt handler for password input and text editor spawning.
#[derive(Debug, Default)]
pub struct TerminalPrompt;

impl TerminalPrompt {
    /// Securely prompts for passphrase with hidden keystrokes.
    ///
    /// # Errors
    /// Returns `io::Error` if reading from terminal fails.
    pub fn prompt_passphrase(prompt_label: &str) -> io::Result<SecretString> {
        eprint!("{prompt_label}");
        io::stderr().flush()?;
        let password = rpassword::read_password()?;
        Ok(SecretString::new(password))
    }

    /// Reads password from an arbitrary `BufRead` reader (useful for automated testing).
    ///
    /// # Errors
    /// Returns `io::Error` if reading from the reader fails.
    #[allow(deprecated)]
    pub fn prompt_passphrase_from_reader<R: BufRead>(reader: &mut R) -> io::Result<SecretString> {
        let password = rpassword::read_password_from_bufread(reader)?;
        Ok(SecretString::new(password))
    }

    /// Detects user's preferred editor ($VISUAL -> $EDITOR -> nano).
    #[must_use]
    pub fn detect_editor() -> String {
        std::env::var("VISUAL")
            .or_else(|_| std::env::var("EDITOR"))
            .unwrap_or_else(|_| "nano".into())
    }

    /// Spawns user's `$EDITOR` on a secure temp file to compose a multi-line reflection.
    ///
    /// The temporary file is created with restricted `0600` permissions on Unix,
    /// read into a zeroized `SecretString`, overwritten with zeros on disk, and deleted.
    ///
    /// # Errors
    /// Returns `io::Error` if creating/opening the temp file, spawning the editor, or reading fails.
    pub fn compose_in_editor(initial_content: Option<&str>) -> io::Result<SecretString> {
        let mut builder = tempfile::Builder::new();
        builder.prefix("roubit_journal_");
        builder.suffix(".md");

        #[cfg(unix)]
        builder.permissions(std::fs::Permissions::from_mode(0o600));

        let mut temp_file = builder.tempfile()?;

        if let Some(initial) = initial_content {
            temp_file.write_all(initial.as_bytes())?;
            temp_file.flush()?;
        }

        let editor = Self::detect_editor();
        let status = Command::new(&editor)
            .arg(temp_file.path())
            .status()
            .map_err(|e| io::Error::other(format!("Failed to spawn editor '{editor}': {e}")))?;

        if !status.success() {
            return Err(io::Error::other(format!(
                "Editor '{editor}' exited with non-zero status"
            )));
        }

        // Read saved content from start of file
        temp_file.seek(SeekFrom::Start(0))?;
        let mut content = String::new();
        temp_file.read_to_string(&mut content)?;

        // Scrub disk contents with zeros before dropping temp file
        let file_len = temp_file.as_file().metadata()?.len();
        if file_len > 0 {
            temp_file.seek(SeekFrom::Start(0))?;
            #[allow(clippy::cast_possible_truncation)]
            let zeros = vec![0u8; file_len as usize];
            temp_file.write_all(&zeros)?;
            temp_file.flush()?;
        }

        Ok(SecretString::new(content.trim().to_string()))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_prompt_passphrase_from_reader() {
        let mut input = Cursor::new(b"my-secret-vault-password\n");
        let secret = TerminalPrompt::prompt_passphrase_from_reader(&mut input).unwrap();
        assert_eq!(secret.as_str(), "my-secret-vault-password");
    }

    #[test]
    fn test_detect_editor_fallback() {
        let editor = TerminalPrompt::detect_editor();
        assert_ne!(editor, "");
    }

    #[test]
    fn test_temp_file_restricted_permissions_and_scrubbing() {
        let mut builder = tempfile::Builder::new();
        builder.prefix("roubit_test_sec_");

        #[cfg(unix)]
        builder.permissions(std::fs::Permissions::from_mode(0o600));

        let mut temp = builder.tempfile().unwrap();

        #[cfg(unix)]
        {
            let metadata = temp.as_file().metadata().unwrap();
            let mode = metadata.permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "temp file must have 0600 permissions");
        }

        temp.write_all(b"sensitive reflection data").unwrap();
        temp.flush().unwrap();

        let len = temp.as_file().metadata().unwrap().len();
        assert!(len > 0);

        // Test overwrite with zeros
        temp.seek(SeekFrom::Start(0)).unwrap();
        #[allow(clippy::cast_possible_truncation)]
        let zeros = vec![0u8; len as usize];
        temp.write_all(&zeros).unwrap();
        temp.flush().unwrap();

        temp.seek(SeekFrom::Start(0)).unwrap();
        let mut read_back = Vec::new();
        temp.read_to_end(&mut read_back).unwrap();
        assert_eq!(read_back, zeros);
    }

    #[test]
    fn test_compose_in_editor_with_true_command() {
        // Temporarily override EDITOR to "true" command (which exits 0 without interaction)
        std::env::set_var("EDITOR", "true");
        let initial = "# Morning Reflection\nFeeling energized and focused.";
        let result = TerminalPrompt::compose_in_editor(Some(initial)).unwrap();
        assert_eq!(result.as_str(), initial);
    }
}
