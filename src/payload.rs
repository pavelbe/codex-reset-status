use jiff::Timestamp;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::error::{CliError, ErrorKind};
use crate::timefmt;

const EXPIRY_KEYS: [&str; 6] = [
    "expires_at",
    "expiresAt",
    "expiry",
    "expires",
    "expiration",
    "expiration_at",
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Credit {
    pub index: usize,
    pub status: Option<String>,
    pub reset_type: Option<String>,
    pub expires_at_utc: Option<String>,
    pub expires_local: Option<String>,
    pub seconds_left: Option<i64>,
    pub time_left: String,
    pub expired: Option<bool>,
}

#[derive(Debug)]
pub struct Summary {
    pub source: &'static str,
    pub checked_at: Timestamp,
    pub available_count: u64,
    pub total_earned_count: Option<u64>,
    pub credits: Vec<Credit>,
    pub warnings: Vec<String>,
}

pub fn parse(payload: &Value, source: &'static str, now: Timestamp) -> Result<Summary, CliError> {
    let root = payload.as_object().ok_or_else(|| {
        CliError::new(
            ErrorKind::Response,
            "endpoint response has an unexpected JSON root",
        )
    })?;
    let credits = find_credits(root, 0).ok_or_else(|| {
        CliError::new(
            ErrorKind::Response,
            "endpoint response has no recognized credits list; the undocumented schema may have changed",
        )
    })?;

    let mut warnings = Vec::new();
    let mut parsed = Vec::with_capacity(credits.len());
    for (position, value) in credits.iter().enumerate() {
        let Some(object) = value.as_object() else {
            warnings.push(format!(
                "credit {} is not an object and was skipped",
                position + 1
            ));
            continue;
        };
        let expiry_value = EXPIRY_KEYS.iter().find_map(|key| object.get(*key));
        let expiry = expiry_value.and_then(timefmt::parse);
        if expiry_value.is_some() && expiry.is_none() {
            warnings.push(format!(
                "credit {} has an invalid expiry value",
                position + 1
            ));
        } else if expiry_value.is_none() {
            warnings.push(format!(
                "credit {} has no recognized expiry field",
                position + 1
            ));
        }

        let seconds_left = expiry.map(|timestamp| timestamp.as_second() - now.as_second());
        parsed.push(Credit {
            index: position + 1,
            status: string_field(object, "status"),
            reset_type: string_field(object, "reset_type"),
            expires_at_utc: expiry.map(|timestamp| timestamp.to_string()),
            expires_local: expiry.map(timefmt::local),
            seconds_left,
            time_left: seconds_left
                .map(timefmt::time_left)
                .unwrap_or_else(|| "unknown".to_owned()),
            expired: seconds_left.map(|seconds| seconds <= 0),
        });
    }

    let available_count = find_u64(root, "available_count", 0).unwrap_or(parsed.len() as u64);
    let total_earned_count = find_u64(root, "total_earned_count", 0);
    if available_count != parsed.len() as u64 {
        warnings.push(format!(
            "available_count is {available_count}, but {} credit rows were parsed",
            parsed.len()
        ));
    }

    Ok(Summary {
        source,
        checked_at: now,
        available_count,
        total_earned_count,
        credits: parsed,
        warnings,
    })
}

fn find_credits(object: &Map<String, Value>, depth: usize) -> Option<&Vec<Value>> {
    if depth > 4 {
        return None;
    }
    for key in ["credits", "data", "items"] {
        match object.get(key) {
            Some(Value::Array(values)) => return Some(values),
            Some(Value::Object(nested)) => {
                if let Some(values) = find_credits(nested, depth + 1) {
                    return Some(values);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_u64(object: &Map<String, Value>, key: &str, depth: usize) -> Option<u64> {
    if depth > 4 {
        return None;
    }
    if let Some(value) = object.get(key) {
        return value
            .as_u64()
            .or_else(|| value.as_str().and_then(|text| text.parse().ok()));
    }
    for container in ["data", "items"] {
        if let Some(Value::Object(nested)) = object.get(container) {
            if let Some(value) = find_u64(nested, key, depth + 1) {
                return Some(value);
            }
        }
    }
    None
}

fn string_field(object: &Map<String, Value>, key: &str) -> Option<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use jiff::Timestamp;
    use serde_json::json;

    use super::parse;

    #[test]
    fn rejects_missing_credit_shape() {
        let now = Timestamp::UNIX_EPOCH;
        let error =
            parse(&json!({"available_count": 0}), "fixture", now).expect_err("must fail closed");
        assert_eq!(error.kind.exit_code(), 6);
    }

    #[test]
    fn never_passes_through_extra_fields() {
        let now: Timestamp = "2026-07-27T00:00:00Z".parse().unwrap();
        let summary = parse(
            &json!({
                "available_count": 1,
                "account_email": "private@example.com",
                "credits": [{
                    "status": "available",
                    "reset_type": "codex_rate_limits",
                    "expires_at": "2026-07-28T00:00:00Z",
                    "access_token": "secret"
                }]
            }),
            "fixture",
            now,
        )
        .unwrap();
        let rendered = serde_json::to_string(&summary.credits).unwrap();
        assert!(!rendered.contains("private@example.com"));
        assert!(!rendered.contains("secret"));
    }
}
