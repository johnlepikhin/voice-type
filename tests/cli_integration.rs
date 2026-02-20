use std::fs;
use std::process::Command;

use tempfile::TempDir;

/// Helper to run voice-type with arguments, returning (exit_code, stdout, stderr).
fn run_voice_type(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_voice-type"))
        .args(args)
        .output()
        .expect("Failed to execute voice-type");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    (code, stdout, stderr)
}

#[test]
fn help_shows_all_commands() {
    let (code, stdout, _stderr) = run_voice_type(&["--help"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("record"));
    assert!(stdout.contains("daemon"));
    assert!(stdout.contains("stop"));
    // status is hidden (not yet implemented)
    assert!(!stdout.contains("status"));
    assert!(stdout.contains("config"));
}

#[test]
fn config_help_shows_subcommands() {
    let (code, stdout, _stderr) = run_voice_type(&["config", "--help"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("validate"));
    assert!(stdout.contains("show"));
    assert!(stdout.contains("init"));
    assert!(stdout.contains("docs"));
}

#[test]
fn config_init_creates_file() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("voice-type.yaml");

    let (code, stdout, _stderr) =
        run_voice_type(&["--config", config_path.to_str().unwrap(), "config", "init"]);

    assert_eq!(code, 0, "config init should succeed");
    assert!(stdout.contains("Config written"));
    assert!(config_path.exists(), "Config file should be created");

    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("provider:"));
    assert!(content.contains("api_key:"));
    assert!(content.contains("sample_rate:"));
}

#[test]
fn config_init_refuses_overwrite() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("voice-type.yaml");

    // Create first
    run_voice_type(&["--config", config_path.to_str().unwrap(), "config", "init"]);
    assert!(config_path.exists());

    // Try to create again without --force
    let (code, _stdout, stderr) =
        run_voice_type(&["--config", config_path.to_str().unwrap(), "config", "init"]);

    assert_ne!(code, 0, "Should fail without --force");
    assert!(
        stderr.contains("already exists") || stderr.contains("--force"),
        "Should mention existing file"
    );
}

#[test]
fn config_init_force_overwrites() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("voice-type.yaml");

    // Create first
    run_voice_type(&["--config", config_path.to_str().unwrap(), "config", "init"]);

    // Force overwrite
    let (code, stdout, _stderr) = run_voice_type(&[
        "--config",
        config_path.to_str().unwrap(),
        "config",
        "init",
        "--force",
    ]);

    assert_eq!(code, 0, "config init --force should succeed");
    assert!(stdout.contains("Config written"));
}

#[test]
fn config_validate_valid() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("voice-type.yaml");

    // Init first
    run_voice_type(&["--config", config_path.to_str().unwrap(), "config", "init"]);

    // Validate
    let (code, stdout, _stderr) = run_voice_type(&[
        "--config",
        config_path.to_str().unwrap(),
        "config",
        "validate",
    ]);

    assert_eq!(code, 0, "Validation should pass for default config");
    assert!(stdout.contains("valid"));
}

#[test]
fn config_validate_bad_config() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("voice-type.yaml");

    // Write invalid config
    fs::write(
        &config_path,
        r#"
provider:
  openai:
    api_key: !FromEnv OPENAI_API_KEY
audio:
  sample_rate: 0
"#,
    )
    .unwrap();

    let (code, _stdout, stderr) = run_voice_type(&[
        "--config",
        config_path.to_str().unwrap(),
        "config",
        "validate",
    ]);

    assert_ne!(code, 0, "Validation should fail for bad sample_rate");
    assert!(
        stderr.contains("sample_rate") || stderr.contains("validation"),
        "Should mention the invalid field"
    );
}

#[test]
fn config_show_displays_yaml() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("voice-type.yaml");

    // Init first
    run_voice_type(&["--config", config_path.to_str().unwrap(), "config", "init"]);

    // Show
    let (code, stdout, _stderr) =
        run_voice_type(&["--config", config_path.to_str().unwrap(), "config", "show"]);

    assert_eq!(code, 0, "config show should succeed");
    assert!(stdout.contains("openai:") || stdout.contains("provider:"));
}

#[test]
fn config_docs_outputs_documentation() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("voice-type.yaml");

    // Docs doesn't need a config file to exist (uses StructDoc)
    // But our impl loads config first... let's init it
    run_voice_type(&["--config", config_path.to_str().unwrap(), "config", "init"]);

    let (code, stdout, _stderr) =
        run_voice_type(&["--config", config_path.to_str().unwrap(), "config", "docs"]);

    assert_eq!(code, 0, "config docs should succeed");
    assert!(!stdout.is_empty(), "Should output documentation");
}

#[test]
fn config_validate_missing_file() {
    let (code, _stdout, stderr) = run_voice_type(&[
        "--config",
        "/tmp/nonexistent-voice-type-test-config.yaml",
        "config",
        "validate",
    ]);

    assert_ne!(code, 0, "Should fail for missing config");
    assert!(
        stderr.contains("not found") || stderr.contains("Failed to load"),
        "Should indicate file not found"
    );
}

#[test]
fn version_flag() {
    let (code, stdout, _stderr) = run_voice_type(&["--version"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("voice-type"));
}
