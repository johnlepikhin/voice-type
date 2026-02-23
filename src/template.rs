//! Template expansion for configuration strings.
//!
//! Supports built-in variables (`{{datetime}}`, `{{date}}`, `{{time}}`)
//! and shell command substitution (`{{$(command)}}`).
//!
//! # Examples
//!
//! ```yaml
//! system_prompt: |
//!   Fix grammar. Current time: {{datetime}}.
//!   Host: {{$(hostname)}}.
//! ```
//!
//! # Security
//!
//! Shell commands are executed via `sh -c` with full shell access.
//! Only use templates in configuration files that you control.

/// Maximum time to wait for a template command to complete.
const COMMAND_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Poll interval when waiting for a template command.
const COMMAND_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);

/// Errors during template expansion.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TemplateError {
    /// Shell command execution failed.
    #[error("Template command `{command}` failed: {reason}")]
    CommandFailed {
        /// The command that failed.
        command: String,
        /// Why it failed.
        reason: String,
    },
}

/// Expand template placeholders in a string.
///
/// Replaces `{{datetime}}`, `{{date}}`, `{{time}}` with local values,
/// and `{{$(cmd)}}` with the stdout of the command. Escaped `\{{`
/// produces a literal `{{`. Unknown variables are left as-is with a
/// warning logged.
///
/// # Errors
///
/// Returns [`TemplateError::CommandFailed`] if a shell command fails.
pub fn expand(template: &str) -> Result<String, TemplateError> {
    // Fast path: no placeholders at all.
    if !template.contains("{{") {
        return Ok(template.to_owned());
    }

    let mut result = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let len = bytes.len();
    // `copied` tracks the byte offset up to which we've already appended to `result`.
    let mut copied = 0;
    let mut i = 0;

    while i < len {
        // Escaped: \{{ → literal {{
        if i + 2 < len && bytes[i] == b'\\' && bytes[i + 1] == b'{' && bytes[i + 2] == b'{' {
            // Flush text before the backslash.
            result.push_str(&template[copied..i]);
            result.push_str("{{");
            i += 3;
            copied = i;
            continue;
        }

        // Opening {{
        if i + 1 < len && bytes[i] == b'{' && bytes[i + 1] == b'{' {
            if let Some(close) = find_closing(bytes, i + 2) {
                // Flush text before {{.
                result.push_str(&template[copied..i]);
                let inner = &template[i + 2..close];
                let expanded = expand_placeholder(inner)?;
                result.push_str(&expanded);
                i = close + 2; // skip }}
                copied = i;
                continue;
            }
            // Unclosed {{ — skip past it so we don't re-match.
            i += 2;
            continue;
        }

        i += 1;
    }

    // Flush remaining tail.
    result.push_str(&template[copied..]);
    Ok(result)
}

/// Find the byte position of `}}` starting from `start`.
fn find_closing(bytes: &[u8], start: usize) -> Option<usize> {
    let len = bytes.len();
    let mut i = start;
    while i + 1 < len {
        if bytes[i] == b'}' && bytes[i + 1] == b'}' {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Expand a single placeholder (the text between `{{` and `}}`).
fn expand_placeholder(inner: &str) -> Result<String, TemplateError> {
    let trimmed = inner.trim();

    // Shell command: $(cmd)
    if let Some(cmd) = trimmed.strip_prefix("$(").and_then(|s| s.strip_suffix(')')) {
        return run_template_command(cmd);
    }

    // Built-in variable
    if let Some(value) = resolve_builtin(trimmed) {
        return Ok(value);
    }

    // Unknown — warn and leave as-is
    tracing::warn!(
        variable = trimmed,
        "Unknown template variable, leaving as-is"
    );
    Ok(format!("{{{{{trimmed}}}}}"))
}

/// Resolve a built-in variable name to its value.
///
/// Uses the `date` command to avoid adding a datetime dependency.
fn resolve_builtin(name: &str) -> Option<String> {
    let fmt = match name {
        "datetime" => "%Y-%m-%dT%H:%M:%S%:z",
        "date" => "%Y-%m-%d",
        "time" => "%H:%M:%S",
        _ => return None,
    };
    // Use date command — no new dependencies needed.
    match run_template_command(&format!("date +'{fmt}'")) {
        Ok(value) => Some(value),
        Err(e) => {
            tracing::warn!(
                variable = name,
                error = %e,
                "Built-in template variable expansion failed (is `date` available?)"
            );
            None
        }
    }
}

/// Execute a shell command and return its trimmed stdout.
fn run_template_command(cmd: &str) -> Result<String, TemplateError> {
    tracing::debug!(command = cmd, "Running template command");
    let mut child = std::process::Command::new("sh")
        .args(["-c", cmd])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|err| TemplateError::CommandFailed {
            command: cmd.to_owned(),
            reason: format!("Failed to spawn: {err}"),
        })?;

    let deadline = std::time::Instant::now() + COMMAND_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(TemplateError::CommandFailed {
                        command: cmd.to_owned(),
                        reason: format!("Timed out after {}s", COMMAND_TIMEOUT.as_secs()),
                    });
                }
                std::thread::sleep(COMMAND_POLL_INTERVAL);
            }
            Err(e) => {
                return Err(TemplateError::CommandFailed {
                    command: cmd.to_owned(),
                    reason: format!("Failed to wait: {e}"),
                })
            }
        }
    };

    if !status.success() {
        let mut stderr = String::new();
        if let Some(mut err_stream) = child.stderr.take() {
            let _ = std::io::Read::read_to_string(&mut err_stream, &mut stderr);
        }
        return Err(TemplateError::CommandFailed {
            command: cmd.to_owned(),
            reason: format!("Exit {status}: {}", stderr.trim()),
        });
    }

    let mut stdout = String::new();
    if let Some(mut out_stream) = child.stdout.take() {
        std::io::Read::read_to_string(&mut out_stream, &mut stdout).map_err(|e| {
            TemplateError::CommandFailed {
                command: cmd.to_owned(),
                reason: format!("Failed to read stdout: {e}"),
            }
        })?;
    }
    Ok(stdout.trim_end().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_placeholders_passthrough() {
        assert_eq!(expand("hello world").unwrap(), "hello world");
    }

    #[test]
    fn empty_string() {
        assert_eq!(expand("").unwrap(), "");
    }

    #[test]
    fn builtin_datetime_format() {
        let result = expand("now: {{datetime}}").unwrap();
        assert!(result.starts_with("now: "));
        // ISO 8601 format: 2026-02-23T14:30:05+03:00
        let dt = &result["now: ".len()..];
        assert_eq!(dt.len(), 25, "unexpected datetime: {dt}");
        assert_eq!(&dt[4..5], "-");
        assert_eq!(&dt[10..11], "T");
    }

    #[test]
    fn builtin_date_format() {
        let result = expand("{{date}}").unwrap();
        assert_eq!(result.len(), 10); // YYYY-MM-DD
        assert_eq!(&result[4..5], "-");
    }

    #[test]
    fn builtin_time_format() {
        let result = expand("{{time}}").unwrap();
        assert_eq!(result.len(), 8); // HH:MM:SS
        assert_eq!(&result[2..3], ":");
    }

    #[test]
    fn shell_command() {
        let result = expand("host: {{$(echo hello)}}").unwrap();
        assert_eq!(result, "host: hello");
    }

    #[test]
    fn shell_command_trims_trailing_newline() {
        let result = expand("{{$(printf 'abc\n\n')}}").unwrap();
        assert_eq!(result, "abc");
    }

    #[test]
    fn shell_command_failure() {
        let result = expand("{{$(false)}}");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("false"));
    }

    #[test]
    fn escaped_braces() {
        let result = expand(r"literal: \{{not expanded}}").unwrap();
        assert_eq!(result, "literal: {{not expanded}}");
    }

    #[test]
    fn unknown_variable_left_as_is() {
        let result = expand("{{unknown_var}}").unwrap();
        assert_eq!(result, "{{unknown_var}}");
    }

    #[test]
    fn multiple_placeholders() {
        let result = expand("a={{$(echo 1)}} b={{$(echo 2)}}").unwrap();
        assert_eq!(result, "a=1 b=2");
    }

    #[test]
    fn unclosed_braces_left_as_is() {
        let result = expand("{{unclosed").unwrap();
        assert_eq!(result, "{{unclosed");
    }

    #[test]
    fn adjacent_braces() {
        let result = expand("}}{{$(echo ok)}}{{").unwrap();
        assert_eq!(result, "}}ok{{");
    }

    #[test]
    fn whitespace_in_placeholder() {
        let result = expand("{{ $(echo trimmed) }}").unwrap();
        assert_eq!(result, "trimmed");
    }

    #[test]
    fn utf8_cyrillic_preserved() {
        let result = expand("Привет {{$(echo мир)}}!").unwrap();
        assert_eq!(result, "Привет мир!");
    }

    #[test]
    fn utf8_no_placeholders() {
        let result = expand("Кириллица без шаблонов").unwrap();
        assert_eq!(result, "Кириллица без шаблонов");
    }

    #[test]
    fn utf8_mixed_with_builtins() {
        let result = expand("Дата: {{date}} конец").unwrap();
        assert!(result.starts_with("Дата: "));
        assert!(result.ends_with(" конец"));
    }
}
