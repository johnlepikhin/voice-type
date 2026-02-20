use clap::{Parser, Subcommand};

/// Voice input for Linux with GTK4 and speech-to-text.
#[derive(Parser, Debug)]
#[command(name = "voice-type", version, about)]
pub struct Cli {
    /// Path to configuration file.
    #[arg(short, long, default_value = "~/.config/voice-type.yaml")]
    pub config: String,

    /// Increase logging verbosity (repeatable: -v, -vv, -vvv).
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Command to execute.
    #[command(subcommand)]
    pub command: Commands,
}

/// Available commands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// One-shot voice recording and transcription.
    Record {
        /// Audio input device (overrides config).
        #[arg(short, long)]
        device: Option<String>,

        /// Language hint, ISO-639-1 (overrides config).
        #[arg(short, long)]
        language: Option<String>,

        /// Recognition prompt (overrides config).
        #[arg(short, long)]
        prompt: Option<String>,
    },

    /// Start background daemon.
    Daemon,

    /// Stop running daemon.
    Stop,

    /// Show daemon status (not yet implemented).
    #[command(hide = true)]
    Status {
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },

    /// Configuration management.
    Config {
        /// Config subcommand.
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

/// Configuration management subcommands.
#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Validate configuration file.
    Validate,

    /// Show current effective configuration.
    Show,

    /// Create default configuration file.
    Init {
        /// Overwrite existing file.
        #[arg(long)]
        force: bool,
    },

    /// Print configuration documentation.
    Docs,
}

impl Cli {
    /// Resolve the config file path, expanding `~` to home directory.
    #[must_use]
    pub fn config_path(&self) -> std::path::PathBuf {
        let path = &self.config;
        if let Some(rest) = path.strip_prefix("~/") {
            if let Ok(home) = std::env::var("HOME") {
                return std::path::PathBuf::from(home).join(rest);
            }
        }
        std::path::PathBuf::from(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_path_expands_tilde() {
        let cli = Cli {
            config: "~/.config/voice-type.yaml".to_owned(),
            verbose: 0,
            command: Commands::Daemon,
        };
        let path = cli.config_path();
        assert!(!path.to_str().unwrap().contains('~'));
    }

    #[test]
    fn config_path_absolute() {
        let cli = Cli {
            config: "/etc/voice-type.yaml".to_owned(),
            verbose: 0,
            command: Commands::Daemon,
        };
        let path = cli.config_path();
        assert_eq!(path.to_str().unwrap(), "/etc/voice-type.yaml");
    }
}
