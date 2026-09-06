//! Client-side tmux session name validator mirroring Go ValidateSessionName.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ValidationError {
    #[error("session name is required")]
    Empty,
    #[error("session name too long")]
    TooLong,
    #[error("session name must not contain ':' or '.'")]
    ContainsColonOrDot,
    #[error("session name must not start with '$'")]
    StartsWithDollar,
    #[error("invalid session name \"{0}\"")]
    InvalidCharacters(String),
}

/// Validates a new/renamed session name against tmux rules.
/// Matches Go backend `ValidateSessionName` in `be/internal/tmux/command.go`:
/// Regex: `^[A-Za-z0-9][A-Za-z0-9_./-]*$`
/// Explicit rules:
/// 1. Empty string rejected ("session name is required")
/// 2. len > 200 rejected ("session name too long")
/// 3. Contains ':' or '.' rejected ("session name must not contain ':' or '.'")
/// 4. Starts with '$' rejected ("session name must not start with '$'")
/// 5. First char must be alphanumeric, subsequent chars can be [A-Za-z0-9_./-] (note: '.' is prohibited by rule 3)
pub fn validate_session_name(name: &str) -> Result<(), ValidationError> {
    if name.is_empty() {
        return Err(ValidationError::Empty);
    }
    if name.len() > 200 {
        return Err(ValidationError::TooLong);
    }
    if name.contains(':') || name.contains('.') {
        return Err(ValidationError::ContainsColonOrDot);
    }
    if name.starts_with('$') {
        return Err(ValidationError::StartsWithDollar);
    }

    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphanumeric() {
        return Err(ValidationError::InvalidCharacters(name.to_string()));
    }

    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '_' || c == '/' || c == '-') {
            return Err(ValidationError::InvalidCharacters(name.to_string()));
        }
    }

    Ok(())
}
