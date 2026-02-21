use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, LazyLock, OnceLock, RwLock};

use serde::{Deserialize, Serialize};
use structdoc::{Documentation, StructDoc};

use crate::error::SecretError;

/// Wrapper around `secstr::SecUtf8` for serde and `StructDoc` support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecUtf8String(secstr::SecUtf8);

impl StructDoc for SecUtf8String {
    fn document() -> Documentation {
        Documentation::leaf("Secret string")
    }
}

impl From<String> for SecUtf8String {
    fn from(v: String) -> Self {
        Self(secstr::SecUtf8::from(v))
    }
}

impl From<&str> for SecUtf8String {
    fn from(v: &str) -> Self {
        Self(secstr::SecUtf8::from(v))
    }
}

impl Deref for SecUtf8String {
    type Target = secstr::SecUtf8;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Cached result of a command execution.
type CommandResult = Result<String, String>;

/// Global cache for command results using `OnceLock` to ensure single execution.
/// Each command gets its own `OnceLock`, so concurrent calls to the same command
/// will block until the first caller completes, then all get the cached result.
static COMMAND_CACHE: LazyLock<RwLock<HashMap<String, Arc<OnceLock<CommandResult>>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Secret value that can be provided as plaintext, from an environment
/// variable, or from a shell command.
///
/// In YAML, use tags to specify the source:
/// ```yaml
/// api_key: !String "raw-key"
/// api_key: !FromEnv OPENAI_API_KEY
/// api_key: !FromCommand "pass show openai"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, StructDoc)]
pub enum Secret {
    /// Plaintext secret string.
    String(SecUtf8String),
    /// Secret string from provided environment variable.
    FromEnv(String),
    /// Secret string from provided command STDOUT.
    ///
    /// # Security
    /// The command is executed via `sh -c` and has full shell access.
    /// Use only with trusted, known commands (e.g., `pass show ...`).
    FromCommand(String),
}

impl Secret {
    /// Create a `Secret::String` from a plain string.
    ///
    /// Intended for tests and configuration defaults.
    #[must_use]
    pub fn from_string(value: &str) -> Self {
        Self::String(SecUtf8String::from(value))
    }

    /// Resolve the secret to a plaintext string.
    ///
    /// # Errors
    /// Returns `SecretError` if the environment variable is not set or the
    /// command fails.
    pub fn unsecure(&self) -> Result<String, SecretError> {
        match self {
            Self::String(v) => Ok(v.unsecure().to_owned()),
            Self::FromEnv(env_var) => {
                std::env::var(env_var).map_err(|_| SecretError::EnvVarNotSet(env_var.clone()))
            }
            Self::FromCommand(command) => {
                // Get or create OnceLock for this command
                let once_lock = {
                    // Try read lock first (fast path for cached commands)
                    if let Some(lock) = COMMAND_CACHE
                        .read()
                        .ok()
                        .and_then(|cache| cache.get(command).cloned())
                    {
                        lock
                    } else {
                        // Need write lock to insert new OnceLock
                        let mut cache = COMMAND_CACHE
                            .write()
                            .map_err(|e| SecretError::CacheError(e.to_string()))?;
                        // Use entry API to avoid double-insertion race
                        cache
                            .entry(command.clone())
                            .or_insert_with(|| Arc::new(OnceLock::new()))
                            .clone()
                    }
                };

                // `get_or_init` guarantees single execution — other threads
                // block here.
                let result = once_lock.get_or_init(|| run_command(command));

                result.clone().map_err(SecretError::CommandFailed)
            }
        }
    }
}

/// Maximum time to wait for a secret command to complete.
const COMMAND_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

/// Poll interval when waiting for a secret command.
const COMMAND_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);

/// Execute a shell command and return its stdout.
///
/// The command is killed if it doesn't complete within [`COMMAND_TIMEOUT`].
fn run_command(command: &str) -> Result<String, String> {
    tracing::debug!("Running secret command {:?}", command);
    let mut child = std::process::Command::new("sh")
        .args(["-c", command])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|err| format!("Failed to run secret command: {err}"))?;

    // Poll with try_wait because std::process::Child has no wait_timeout in stable Rust.
    let deadline = std::time::Instant::now() + COMMAND_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "Secret command timed out after {}s",
                        COMMAND_TIMEOUT.as_secs()
                    ));
                }
                std::thread::sleep(COMMAND_POLL_INTERVAL);
            }
            Err(e) => return Err(format!("Failed to wait for secret command: {e}")),
        }
    };

    if !status.success() {
        let mut stderr = String::new();
        if let Some(mut err_stream) = child.stderr.take() {
            let _ = std::io::Read::read_to_string(&mut err_stream, &mut stderr);
        }
        return Err(format!(
            "Secret command failed with status {status}: {stderr}"
        ));
    }

    let mut stdout = String::new();
    if let Some(mut out_stream) = child.stdout.take() {
        std::io::Read::read_to_string(&mut out_stream, &mut stdout)
            .map_err(|e| format!("Failed to read command output: {e}"))?;
    }
    Ok(stdout.trim_end().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_string_roundtrip() {
        let secret = Secret::String(SecUtf8String::from("test-key"));
        let yaml = serde_yaml::to_string(&secret).unwrap();
        let parsed: Secret = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.unsecure().unwrap(), "test-key");
    }

    #[test]
    fn secret_from_env() {
        // Use HOME which is guaranteed to exist, avoiding unsafe set_var.
        let secret = Secret::FromEnv("HOME".to_owned());
        let result = secret.unsecure().unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn secret_from_env_missing() {
        let secret = Secret::FromEnv("VOICE_TYPE_NONEXISTENT_VAR_12345".to_owned());
        assert!(secret.unsecure().is_err());
    }

    #[test]
    fn secret_from_command() {
        let secret = Secret::FromCommand("echo hello-from-cmd".to_owned());
        assert_eq!(secret.unsecure().unwrap(), "hello-from-cmd");
    }

    #[test]
    fn secret_yaml_tags() {
        let yaml = "!FromEnv OPENAI_API_KEY";
        let secret: Secret = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(secret, Secret::FromEnv(ref s) if s == "OPENAI_API_KEY"));
    }
}
