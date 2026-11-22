use std::env;
use std::io::{self, Read};
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use serde::Serialize;
use serde_json::Value;
use utter_core::providers::{ProviderId, Role};
use utter_core::settings::Settings;

const SUBCOMMANDS: [&str; 9] = [
    "get",
    "set",
    "status",
    "key",
    "help",
    "--help",
    "-h",
    "--version",
    "-V",
];

#[derive(Parser)]
#[command(
    name = "utter",
    version,
    about = "Read and change Utter's settings. The running app applies every change right away."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(
        about = "Print all settings, or the one named, as JSON",
        after_help = "Names follow the JSON, for example transcription.provider or cleanup.enabled."
    )]
    Get { name: Option<String> },
    #[command(
        about = "Change one setting",
        after_help = "Examples:\n  utter set transcription.provider elevenlabs\n  utter set transcription.apiKeyVariable MY_ELEVENLABS_KEY\n  utter set cleanup.enabled false\n  utter set vocabulary '[\"Tauri\", \"Siobhan\"]'\n\nValues are read as JSON when they parse (true, null, 3, [...]) and as text otherwise.\nChoosing a provider also sets its address and default model, and resets apiKeyVariable\nto that provider's usual variable."
    )]
    Set { name: String, value: String },
    #[command(about = "Show the settings file and where each API key comes from")]
    Status {
        #[arg(long, help = "Print the status as JSON")]
        json: bool,
    },
    #[command(
        about = "Save or remove an API key in the macOS Keychain",
        after_help = "Utter reads each key from the environment variable named by apiKeyVariable first\n(GROQ_API_KEY, ELEVENLABS_API_KEY, ...), and only then from the Keychain."
    )]
    Key {
        #[command(subcommand)]
        action: KeyAction,
    },
}

#[derive(Subcommand)]
enum KeyAction {
    #[command(about = "Save the key read from standard input")]
    Set {
        provider: String,
        #[arg(long, default_value = "transcription")]
        role: String,
    },
    #[command(about = "Remove a saved key")]
    Remove {
        provider: String,
        #[arg(long, default_value = "transcription")]
        role: String,
    },
}

struct Paths {
    settings: PathBuf,
    keychain_service: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    version: &'static str,
    settings_file: PathBuf,
    settings_error: Option<String>,
    transcription: EndpointStatus,
    cleanup: Option<EndpointStatus>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EndpointStatus {
    provider: ProviderId,
    model: String,
    api_key_variable: Option<String>,
    variable_set_for_apps: bool,
}

pub fn run_if_requested(identifier: &str) -> Option<i32> {
    let first = env::args().nth(1)?;
    if !SUBCOMMANDS.contains(&first.as_str()) {
        return None;
    }
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let _ = error.print();
            return Some(error.exit_code());
        }
    };
    let paths = match dirs::config_dir() {
        Some(config) => Paths {
            settings: config.join(identifier).join("settings.json"),
            keychain_service: identifier.to_owned(),
        },
        None => {
            eprintln!("utter: could not find the settings folder");
            return Some(1);
        }
    };
    match execute(cli.command, &paths) {
        Ok(()) => Some(0),
        Err(message) => {
            eprintln!("utter: {message}");
            Some(1)
        }
    }
}

fn execute(command: Command, paths: &Paths) -> Result<(), String> {
    match command {
        Command::Get { name } => {
            let settings = Settings::load(&paths.settings).map_err(|error| error.to_string())?;
            let value = match name {
                Some(name) => settings
                    .value_at(&name)
                    .map_err(|error| error.to_string())?,
                None => serde_json::to_value(&settings).map_err(|error| error.to_string())?,
            };
            println!("{}", to_pretty_json(&value)?);
        }
        Command::Set { name, value } => {
            let current = Settings::load(&paths.settings).map_err(|error| error.to_string())?;
            let next = current
                .with_value(&name, &value)
                .map_err(|error| error.to_string())?;
            let changes = describe_changes(&current, &next)?;
            if changes.is_empty() {
                println!("Nothing changed: {name} already has that value.");
                return Ok(());
            }
            next.save(&paths.settings)
                .map_err(|error| error.to_string())?;
            for change in changes {
                println!("{change}");
            }
            println!("Saved. Utter applies it right away if it is running.");
        }
        Command::Status { json } => {
            let status = read_status(paths);
            if json {
                println!("{}", to_pretty_json(&status)?);
            } else {
                print_status(&status);
            }
        }
        Command::Key { action } => match action {
            KeyAction::Set { provider, role } => {
                let (provider, role) = parse_provider_and_role(&provider, &role)?;
                let mut key = String::new();
                io::stdin().read_to_string(&mut key).map_err(|error| {
                    format!("could not read the key from standard input: {error}")
                })?;
                let key = key.trim();
                if key.is_empty() {
                    return Err("standard input was empty; pipe the key in, for example `utter key set groq < key.txt`".to_owned());
                }
                keychain_entry(paths, provider, role)?
                    .set_password(key)
                    .map_err(|error| format!("could not save the key: {error}"))?;
                println!("Saved the {} key in the Keychain.", provider.preset().label);
            }
            KeyAction::Remove { provider, role } => {
                let (provider, role) = parse_provider_and_role(&provider, &role)?;
                match keychain_entry(paths, provider, role)?.delete_credential() {
                    Ok(()) | Err(keyring::Error::NoEntry) => {
                        println!(
                            "Removed the {} key from the Keychain.",
                            provider.preset().label
                        );
                    }
                    Err(error) => return Err(format!("could not remove the key: {error}")),
                }
            }
        },
    }
    Ok(())
}

fn read_status(paths: &Paths) -> Status {
    let checked = Settings::load_and_validate(&paths.settings);
    let settings_error = checked.as_ref().err().map(ToString::to_string);
    let settings = checked
        .or_else(|_| Settings::load(&paths.settings))
        .unwrap_or_default();
    let transcription_variable = settings.transcription.resolve_api_key_variable();
    let cleanup_variable = settings.cleanup.resolve_api_key_variable();
    Status {
        version: env!("CARGO_PKG_VERSION"),
        settings_file: paths.settings.clone(),
        settings_error,
        transcription: EndpointStatus {
            provider: settings.transcription.provider,
            model: settings.transcription.model.clone(),
            variable_set_for_apps: is_set_for_apps(transcription_variable.as_deref()),
            api_key_variable: transcription_variable,
        },
        cleanup: settings.cleanup.enabled.then(|| EndpointStatus {
            provider: settings.cleanup.provider,
            model: settings.cleanup.model.clone(),
            variable_set_for_apps: is_set_for_apps(cleanup_variable.as_deref()),
            api_key_variable: cleanup_variable,
        }),
    }
}

fn print_status(status: &Status) {
    println!("Utter {}", status.version);
    println!("Settings file: {}", status.settings_file.display());
    if let Some(error) = &status.settings_error {
        println!("  Problem: {error}");
        println!("  Utter keeps its last good settings until this is fixed.");
    }
    print_endpoint("Speech to text", Some(&status.transcription));
    print_endpoint("Cleanup", status.cleanup.as_ref());
}

fn print_endpoint(title: &str, endpoint: Option<&EndpointStatus>) {
    let Some(endpoint) = endpoint else {
        println!("{title}: off");
        return;
    };
    println!(
        "{title}: {}, model {}",
        endpoint.provider.preset().label,
        endpoint.model
    );
    match &endpoint.api_key_variable {
        Some(variable) if endpoint.variable_set_for_apps => {
            println!("  API key: from {variable}, which apps opened from the Dock can read");
        }
        Some(variable) => println!(
            "  API key: {variable} is not set for apps opened from the Dock, so Utter uses the Keychain"
        ),
        None => println!("  API key: from the Keychain, if one is saved"),
    }
}

#[cfg(target_os = "macos")]
fn is_set_for_apps(variable: Option<&str>) -> bool {
    let Some(variable) = variable else {
        return false;
    };
    std::process::Command::new("launchctl")
        .args(["getenv", variable])
        .output()
        .is_ok_and(|output| !String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

#[cfg(not(target_os = "macos"))]
fn is_set_for_apps(variable: Option<&str>) -> bool {
    variable.is_some_and(|variable| env::var(variable).is_ok_and(|value| !value.trim().is_empty()))
}

fn parse_provider_and_role(provider: &str, role: &str) -> Result<(ProviderId, Role), String> {
    let provider: ProviderId = serde_json::from_value(Value::String(provider.to_owned()))
        .map_err(|error| format!("provider: {error}"))?;
    let role: Role = serde_json::from_value(Value::String(role.to_owned()))
        .map_err(|error| format!("role: {error}"))?;
    Ok((provider, role))
}

fn keychain_entry(
    paths: &Paths,
    provider: ProviderId,
    role: Role,
) -> Result<keyring::Entry, String> {
    keyring::Entry::new(&paths.keychain_service, &provider.api_key_slot(role))
        .map_err(|error| format!("could not open the Keychain: {error}"))
}

fn describe_changes(before: &Settings, after: &Settings) -> Result<Vec<String>, String> {
    let before = serde_json::to_value(before).map_err(|error| error.to_string())?;
    let after = serde_json::to_value(after).map_err(|error| error.to_string())?;
    let mut changes = Vec::new();
    collect_changes(&before, &after, "", &mut changes);
    Ok(changes)
}

fn collect_changes(before: &Value, after: &Value, path: &str, changes: &mut Vec<String>) {
    match (before, after) {
        (Value::Object(before_fields), Value::Object(after_fields)) => {
            for (name, after_value) in after_fields {
                let child = if path.is_empty() {
                    name.clone()
                } else {
                    format!("{path}.{name}")
                };
                let before_value = before_fields.get(name).unwrap_or(&Value::Null);
                collect_changes(before_value, after_value, &child, changes);
            }
        }
        _ if before != after => changes.push(format!("{path}: {before} -> {after}")),
        _ => {}
    }
}

fn to_pretty_json(value: &impl Serialize) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_each_changed_setting_by_its_name() {
        let before = Settings::default();
        let after = before
            .with_value("transcription.provider", "elevenlabs")
            .expect("sets");
        let changes = describe_changes(&before, &after).expect("describes");
        assert_eq!(
            changes,
            vec![
                r#"transcription.baseUrl: "https://api.groq.com/openai/v1" -> "https://api.elevenlabs.io/v1""#,
                r#"transcription.model: "whisper-large-v3" -> "scribe_v3""#,
                r#"transcription.provider: "groq" -> "elevenlabs""#,
            ]
        );
    }

    #[test]
    fn parses_subcommands_and_leaves_other_launches_to_the_app() {
        assert!(Cli::try_parse_from(["utter", "set", "trigger", "toggle"]).is_ok());
        assert!(Cli::try_parse_from(["utter", "status", "--json"]).is_ok());
        assert!(Cli::try_parse_from(["utter", "key", "set", "groq"]).is_ok());
        assert!(!SUBCOMMANDS.contains(&"-psn_0_12345"));
    }

    #[test]
    fn rejects_unknown_providers_with_the_valid_names() {
        let error = parse_provider_and_role("elevenlab", "transcription").expect_err("fails");
        assert!(error.contains("elevenlabs"));
    }
}
