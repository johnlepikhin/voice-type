#![warn(clippy::all, clippy::pedantic)]

mod app;
mod cli;

use anyhow::{Context, Result};
use clap::Parser;
use gtk4::gio;
use gtk4::prelude::*;

const DAEMON_APP_ID: &str = "com.voicetype.daemon";

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    let log_level = match cli.verbose {
        0 => tracing::Level::WARN,
        1 => tracing::Level::INFO,
        2 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive(log_level.into()),
        )
        .init();

    match cli.command {
        cli::Commands::Record {
            ref device,
            ref language,
            ref prompt,
        } => cmd_record(&cli, device.clone(), language.clone(), prompt.clone()),
        cli::Commands::Daemon => cmd_daemon(&cli),
        cli::Commands::Stop => {
            cmd_stop();
            Ok(())
        }
        cli::Commands::Status { .. } => {
            tracing::warn!("Status command not yet implemented");
            Ok(())
        }
        cli::Commands::Config { ref command } => cmd_config(&cli, command),
    }
}

/// Execute the `record` command: open a GTK window for one-shot recording.
fn cmd_record(
    cli: &cli::Cli,
    device: Option<String>,
    language: Option<String>,
    prompt: Option<String>,
) -> Result<()> {
    let config_path = cli.config_path();
    let mut config = voice_type::config::AppConfig::load(&config_path)
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

    let application = gtk4::Application::builder()
        .application_id("com.voicetype.record")
        .build();

    let config_for_activate = config;
    application.connect_activate(move |app| {
        app::load_css();
        let window = app::build_recording_window(app, &config_for_activate);
        window.present();
    });

    application.run_with_args::<String>(&[]);
    Ok(())
}

/// Execute the `daemon` command: run background service with hotkey listener.
fn cmd_daemon(cli: &cli::Cli) -> Result<()> {
    let config_path = cli.config_path();
    let config = voice_type::config::AppConfig::load(&config_path)
        .with_context(|| format!("Failed to load config from {}", config_path.display()))?;
    config
        .validate()
        .context("Configuration validation failed")?;

    let application = gtk4::Application::builder()
        .application_id(DAEMON_APP_ID)
        .flags(gio::ApplicationFlags::IS_SERVICE)
        .build();

    let config_for_startup = config;
    let initialized = std::cell::Cell::new(false);

    // Use `startup` for initialization — IS_SERVICE apps don't auto-activate.
    application.connect_startup(move |app_ref| {
        if initialized.get() {
            return;
        }
        initialized.set(true);

        app::load_css();
        app::run_daemon(app_ref, &config_for_startup);
    });

    // Re-activation from `voice-type stop` → shut down gracefully.
    application.connect_activate(|app_ref| {
        tracing::info!("Received stop signal, shutting down...");
        app_ref.quit();
    });

    // Hold the application so it stays alive.
    let _hold_guard = application.hold();

    application.run_with_args::<String>(&[]);
    Ok(())
}

/// Execute the `stop` command: send quit signal to running daemon via D-Bus.
fn cmd_stop() {
    let app = gio::Application::new(Some(DAEMON_APP_ID), gio::ApplicationFlags::empty());

    if let Err(e) = app.register(None::<&gio::Cancellable>) {
        eprintln!("Cannot connect to D-Bus: {e}");
        return;
    }

    if app.is_remote() {
        // Sending activate() to the primary instance triggers its
        // re-activation handler, which calls quit().
        app.activate();
        println!("Stopping voice-type daemon...");
    } else {
        println!("No running daemon found.");
    }
}

/// Execute config subcommands.
fn cmd_config(cli: &cli::Cli, command: &cli::ConfigCommands) -> Result<()> {
    match command {
        cli::ConfigCommands::Validate => {
            let config_path = cli.config_path();
            let config = voice_type::config::AppConfig::load(&config_path)
                .with_context(|| format!("Failed to load config from {}", config_path.display()))?;
            config.validate().context("Validation failed")?;
            println!("Configuration is valid.");
            Ok(())
        }
        cli::ConfigCommands::Show => {
            let config_path = cli.config_path();
            let config = voice_type::config::AppConfig::load(&config_path)
                .with_context(|| format!("Failed to load config from {}", config_path.display()))?;
            let yaml = serde_yaml::to_string(&config).context("Failed to serialize config")?;
            print!("{yaml}");
            Ok(())
        }
        cli::ConfigCommands::Init { force } => {
            let config_path = cli.config_path();
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
            std::fs::write(&config_path, voice_type::config::AppConfig::default_yaml())
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
