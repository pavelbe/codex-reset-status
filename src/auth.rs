use std::env;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::{CliError, ErrorKind};
use crate::secret::Secret;

const MAX_AUTH_BYTES: u64 = 1024 * 1024;

pub struct Auth {
    pub token: Secret,
    pub account_id: Option<String>,
}

pub fn resolve_path(override_path: Option<&Path>) -> Result<PathBuf, CliError> {
    if let Some(path) = override_path {
        return Ok(path.to_path_buf());
    }
    if let Some(path) = env::var_os("CODEX_RESET_STATUS_AUTH_FILE") {
        return Ok(PathBuf::from(path));
    }
    if let Some(home) = env::var_os("CODEX_HOME") {
        return Ok(PathBuf::from(home).join("auth.json"));
    }
    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .ok_or_else(|| CliError::new(ErrorKind::Auth, "cannot determine the home directory"))?;
    Ok(PathBuf::from(home).join(".codex").join("auth.json"))
}

pub fn read(path: &Path) -> Result<Auth, CliError> {
    let file = File::open(path).map_err(|error| {
        CliError::new(
            ErrorKind::Auth,
            format!("cannot open auth file {}: {error}", path.display()),
        )
    })?;
    let mut bytes = Vec::new();
    file.take(MAX_AUTH_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            CliError::new(
                ErrorKind::Auth,
                format!("cannot read auth file {}: {error}", path.display()),
            )
        })?;
    if bytes.len() as u64 > MAX_AUTH_BYTES {
        return Err(CliError::new(
            ErrorKind::Auth,
            format!("auth file exceeds {MAX_AUTH_BYTES} bytes"),
        ));
    }

    let value: Value = serde_json::from_slice(&bytes).map_err(|error| {
        CliError::new(
            ErrorKind::Auth,
            format!("auth file is not valid JSON: {error}"),
        )
    })?;
    let object = value
        .as_object()
        .ok_or_else(|| CliError::new(ErrorKind::Auth, "auth file has an unexpected JSON root"))?;
    let tokens = object
        .get("tokens")
        .and_then(Value::as_object)
        .unwrap_or(object);

    let token = tokens
        .get("access_token")
        .or_else(|| object.get("access_token"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| CliError::new(ErrorKind::Auth, "auth file has no access_token"))?;

    let account_id = ["account_id", "chatgpt_account_id"]
        .iter()
        .find_map(|key| {
            tokens
                .get(*key)
                .or_else(|| object.get(*key))
                .and_then(Value::as_str)
        })
        .filter(|value| !value.is_empty())
        .map(str::to_owned);

    Ok(Auth {
        token: Secret::new(token.to_owned()),
        account_id,
    })
}
