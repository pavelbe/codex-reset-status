#![forbid(unsafe_code)]

mod auth;
mod cli;
mod error;
mod http;
mod payload;
mod render;
mod secret;
mod timefmt;

use std::fs::File;
use std::io::Read;
use std::process::ExitCode;

use jiff::Timestamp;
use serde_json::Value;

use crate::cli::{Action, Config};
use crate::error::{CliError, ErrorKind};

const MAX_FIXTURE_BYTES: u64 = 1024 * 1024;

fn main() -> ExitCode {
    std::panic::set_hook(Box::new(|_| {
        eprintln!("codex-reset-status: internal error");
    }));

    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("codex-reset-status: {error}");
            ExitCode::from(error.kind.exit_code() as u8)
        }
    }
}

fn execute() -> Result<(), CliError> {
    match cli::parse()? {
        Action::Help => {
            println!("{}", cli::help());
            Ok(())
        }
        Action::Version => {
            println!("codex-reset-status {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Action::Run(config) => run(config),
    }
}

fn run(config: Config) -> Result<(), CliError> {
    let (value, source) = if let Some(path) = &config.fixture {
        (read_fixture(path)?, "fixture")
    } else {
        let path = auth::resolve_path(config.auth_file.as_deref())?;
        let auth = auth::read(&path)?;
        (
            http::fetch(&config.endpoint, config.timeout_secs, &auth)?,
            "live",
        )
    };
    let summary = payload::parse(&value, source, Timestamp::now())?;
    if config.json {
        println!(
            "{}",
            render::json(&summary).map_err(|error| {
                CliError::new(
                    ErrorKind::Internal,
                    format!("cannot serialize safe JSON output: {error}"),
                )
            })?
        );
    } else {
        println!("{}", render::text(&summary));
    }
    Ok(())
}

fn read_fixture(path: &std::path::Path) -> Result<Value, CliError> {
    let file = File::open(path).map_err(|error| {
        CliError::new(
            ErrorKind::Response,
            format!("cannot open fixture {}: {error}", path.display()),
        )
    })?;
    let mut bytes = Vec::new();
    file.take(MAX_FIXTURE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            CliError::new(
                ErrorKind::Response,
                format!("cannot read fixture {}: {error}", path.display()),
            )
        })?;
    if bytes.len() as u64 > MAX_FIXTURE_BYTES {
        return Err(CliError::new(
            ErrorKind::Response,
            format!("fixture exceeds {MAX_FIXTURE_BYTES} bytes"),
        ));
    }
    serde_json::from_slice(&bytes).map_err(|error| {
        CliError::new(
            ErrorKind::Response,
            format!("fixture is not valid JSON: {error}"),
        )
    })
}
