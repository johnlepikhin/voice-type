#![warn(clippy::all, clippy::pedantic)]

mod app;
mod cli;

use anyhow::{Context, Result};
use clap::Parser;
use gtk4::prelude::*;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    let log_level = match cli.verbose {
        0 => tracing::Level::WARN,
        1 => tracing::Level::INFO,
        2 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level.to_string()));

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let config_path = cli.config_path();
    match cli.command {
        cli::Commands::Record {
            device,
            language,
            prompt,
        } => cmd_record(&config_path, device, language, prompt),
        cli::Commands::Config { command } => cmd_config(&config_path, &command),
    }
}

/// Execute the `record` command: show overlay, record, transcribe, print to stdout.
fn cmd_record(
    config_path: &std::path::Path,
    device: Option<String>,
    language: Option<String>,
    prompt: Option<String>,
) -> Result<()> {
    let mut config = voice_type::config::AppConfig::load(config_path)
        .with_context(|| format!("Failed to load config from {}", config_path.display()))?;

    // Apply CLI overrides
    if let Some(dev) = device {
        config.audio.device = Some(dev);
    }
    match &mut config.provider {
        voice_type::config::ProviderConfig::OpenAi(ref mut c) => {
            if let Some(lang) = language {
                let lc = voice_type::types::LanguageCode::new(&lang)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                c.language = Some(lc);
            }
            if let Some(p) = prompt {
                c.prompt = Some(p);
            }
        }
        _ => anyhow::bail!("Unsupported provider for CLI overrides"),
    }

    config
        .validate()
        .context("Configuration validation failed")?;

    tracing::debug!(?config_path, "Config loaded");

    let application = gtk4::Application::builder()
        .application_id("com.voicetype.record")
        .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
        .build();

    tracing::debug!(
        app_id = application.application_id().map(|s| s.to_string()).as_deref(),
        flags = ?application.flags(),
        is_registered = application.is_registered(),
        is_remote = application.is_remote(),
        "GApplication created"
    );

    let exit_code = std::rc::Rc::new(std::cell::Cell::new(0i32));

    let config_for_activate = config;
    let exit_code_for_activate = std::rc::Rc::clone(&exit_code);
    application.connect_activate(move |app| {
        tracing::debug!(
            is_registered = app.is_registered(),
            is_remote = app.is_remote(),
            "GTK activate signal fired"
        );
        app::load_css();
        app::run_record(app, &config_for_activate, &exit_code_for_activate);
        tracing::debug!(
            window_count = app.windows().len(),
            "run_record returned, activate handler done"
        );
    });

    application.connect_startup(|app| {
        tracing::debug!(
            is_registered = app.is_registered(),
            is_remote = app.is_remote(),
            "GTK startup signal fired"
        );
    });

    tracing::debug!("Calling hold() + run_with_args");
    let _hold = application.hold();
    let gtk_exit = application.run_with_args(&["voice-type"]);
    tracing::debug!(
        ?gtk_exit,
        app_exit = exit_code.get(),
        "GTK main loop exited"
    );

    let code = exit_code.get();
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}

/// Execute config subcommands.
fn cmd_config(config_path: &std::path::Path, command: &cli::ConfigCommands) -> Result<()> {
    match command {
        cli::ConfigCommands::Validate => {
            let config = voice_type::config::AppConfig::load(config_path)
                .with_context(|| format!("Failed to load config from {}", config_path.display()))?;
            config.validate().context("Validation failed")?;
            println!("Configuration is valid.");
            Ok(())
        }
        cli::ConfigCommands::Show => {
            let config = voice_type::config::AppConfig::load(config_path)
                .with_context(|| format!("Failed to load config from {}", config_path.display()))?;
            let yaml = serde_yaml::to_string(&config).context("Failed to serialize config")?;
            print!("{yaml}");
            Ok(())
        }
        cli::ConfigCommands::Init { force } => {
            if config_path.exists() && !*force {
                anyhow::bail!(
                    "Config file already exists at {}. Use --force to overwrite.",
                    config_path.display()
                );
            }
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory {}", parent.display()))?;
            }
            std::fs::write(config_path, voice_type::config::AppConfig::default_yaml())
                .with_context(|| format!("Failed to write config to {}", config_path.display()))?;
            println!("Config written to {}", config_path.display());
            Ok(())
        }
        cli::ConfigCommands::Docs => {
            println!("{}", voice_type::config::AppConfig::docs());
            Ok(())
        }
    }
}
