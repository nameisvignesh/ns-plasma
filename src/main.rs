mod config;
mod provider;
mod tools;
mod ui;

use std::{path::PathBuf, process::ExitCode};

use clap::{Args, Parser, Subcommand};

use config::AppConfig;
use provider::{ChatMessage, ProviderClient, ProviderSettings};

const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const LOGO_BYTES: &[u8] = include_bytes!("../assets/logo.png");

#[derive(Debug, Parser)]
#[command(name = "plasma", version = VERSION, about = "NS Plasma — a Rust-native coding workspace")]
struct Cli {
    #[arg(short = 'p', long = "prompt", help = "Run one headless agent prompt")]
    prompt: Option<String>,
    #[arg(long, help = "Override the configured model for this prompt")]
    model: Option<String>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    App,
    Init,
    Doctor,
    Provider {
        #[command(subcommand)]
        command: ProviderCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ProviderCommand {
    List,
    Set(ProviderSetArgs),
    Test,
    Models,
}

#[derive(Debug, Args)]
struct ProviderSetArgs {
    #[arg(long)]
    base_url: String,
    #[arg(long)]
    model: String,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    api_key_env: Option<String>,
}

fn workspace() -> Result<PathBuf, String> {
    std::env::current_dir().map_err(|error| format!("could not determine workspace: {error}"))
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("plasma: {error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    let root = workspace()?;
    if let Some(prompt) = cli.prompt {
        return run_headless(&root, &prompt, cli.model.as_deref());
    }
    match cli.command {
        Some(Command::App) | None => {
            let config = AppConfig::load(&root).map_err(|error| error.to_string())?;
            ui::run(&root, config).map_err(|error| error.to_string())
        }
        Some(Command::Init) => init(&root),
        Some(Command::Doctor) => doctor(&root),
        Some(Command::Provider { command }) => provider_command(&root, command),
    }
}

fn run_headless(root: &PathBuf, prompt: &str, model_override: Option<&str>) -> Result<(), String> {
    let config = AppConfig::load(root).map_err(|error| error.to_string())?;
    let mut provider = config.provider.ok_or_else(|| {
        "no provider configured; run `plasma provider set --base-url <url> --model <id>` first"
            .to_owned()
    })?;
    if let Some(model) = model_override {
        provider.model = model.to_owned();
    }
    let client = ProviderClient::new(provider).map_err(|error| error.to_string())?;
    let messages = vec![ChatMessage {
        role: "user".to_owned(),
        content: prompt.to_owned(),
    }];
    let answer = client.chat(&messages).map_err(|error| error.to_string())?;
    println!("{answer}");
    Ok(())
}

fn init(root: &PathBuf) -> Result<(), String> {
    let path = root.join("PLASMA.md");
    if !path.exists() {
        std::fs::write(
            &path,
            "# Plasma Project Context\n\nAdd project-specific instructions here.\n",
        )
        .map_err(|error| error.to_string())?;
    }
    let config = AppConfig::load(root).map_err(|error| error.to_string())?;
    config.save(root).map_err(|error| error.to_string())?;
    println!("Initialized NS Plasma in {}", root.display());
    Ok(())
}

fn doctor(root: &PathBuf) -> Result<(), String> {
    let config_path = AppConfig::path(root);
    let config = AppConfig::load(root).map_err(|error| error.to_string())?;
    println!("NS Plasma {VERSION}");
    println!("workspace: {}", root.display());
    println!(
        "config: {} ({})",
        config_path.display(),
        if config_path.exists() {
            "present"
        } else {
            "defaults"
        }
    );
    match config.provider {
        Some(provider) => {
            println!("provider: {}", provider.name);
            println!("endpoint: {}", provider.base_url);
            println!(
                "model: {}",
                if provider.model.is_empty() {
                    "not configured"
                } else {
                    &provider.model
                }
            );
            println!(
                "api key: {}",
                provider
                    .api_key_env
                    .as_deref()
                    .unwrap_or("none (suitable for local endpoints)")
            );
        }
        None => println!("provider: not configured (honest empty state)"),
    }
    Ok(())
}

fn provider_command(root: &PathBuf, command: ProviderCommand) -> Result<(), String> {
    let mut config = AppConfig::load(root).map_err(|error| error.to_string())?;
    match command {
        ProviderCommand::List => match config.provider {
            Some(provider) => println!(
                "{} — {} — {}",
                provider.name,
                provider.base_url,
                if provider.model.is_empty() {
                    "model not selected"
                } else {
                    &provider.model
                }
            ),
            None => println!("No providers configured."),
        },
        ProviderCommand::Set(args) => {
            config.provider = Some(ProviderSettings {
                name: args
                    .name
                    .unwrap_or_else(|| "OpenAI-compatible provider".to_owned()),
                base_url: args.base_url,
                model: args.model,
                api_key_env: args.api_key_env,
            });
            config.save(root).map_err(|error| error.to_string())?;
            println!("Provider saved without storing an API key.");
        }
        ProviderCommand::Test => {
            let provider = config
                .provider
                .ok_or_else(|| "no provider configured".to_owned())?;
            ProviderClient::new(provider)
                .map_err(|error| error.to_string())?
                .test_connection()
                .map_err(|error| error.to_string())?;
            println!("Provider connection succeeded.");
        }
        ProviderCommand::Models => {
            let provider = config
                .provider
                .ok_or_else(|| "no provider configured".to_owned())?;
            let models = ProviderClient::new(provider)
                .map_err(|error| error.to_string())?
                .list_models()
                .map_err(|error| error.to_string())?;
            if models.is_empty() {
                println!("Provider returned no models.");
            } else {
                for model in models {
                    println!("{model}");
                }
            }
        }
    }
    Ok(())
}
