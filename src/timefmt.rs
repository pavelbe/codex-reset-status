use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde_json::Value;

pub fn parse(value: &Value) -> Option<Timestamp> {
    match value {
        Value::Number(number) => number.as_f64().and_then(parse_number),
        Value::String(text) => parse_text(text),
        _ => None,
    }
}

fn parse_text(text: &str) -> Option<Timestamp> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(timestamp) = text.parse::<Timestamp>() {
        return Some(timestamp);
    }
    if let Ok(number) = text.parse::<f64>() {
        return parse_number(number);
    }

    let time_part = text.get(10..).unwrap_or_default();
    let has_offset = text.ends_with('Z') || time_part.contains('+') || time_part.contains('-');
    if !has_offset {
        return format!("{text}Z").parse::<Timestamp>().ok();
    }
    None
}

fn parse_number(value: f64) -> Option<Timestamp> {
    if !value.is_finite() {
        return None;
    }
    if value.abs() > 10_000_000_000.0 {
        Timestamp::from_millisecond(value.trunc() as i64).ok()
    } else {
        let seconds = value.trunc() as i64;
        let nanos = ((value.fract()) * 1_000_000_000.0).trunc() as i32;
        Timestamp::new(seconds, nanos).ok()
    }
}

/// Resolves the display zone. Without `--utc` this is the host zone from `TZ` or
/// `/etc/localtime`; when that cannot be resolved the caller gets UTC plus a
/// warning, so a fallback is never silent.
pub fn resolve_zone(utc: bool) -> (TimeZone, Option<String>) {
    if utc {
        return (TimeZone::UTC, None);
    }
    match TimeZone::try_system() {
        Ok(zone) => (zone, None),
        Err(error) => (
            TimeZone::UTC,
            Some(format!(
                "cannot determine the system time zone ({error}); times are shown in UTC"
            )),
        ),
    }
}

/// Human-readable zone label for output, for example `Europe/Moscow` or `UTC`.
pub fn zone_label(zone: &TimeZone) -> String {
    zone.iana_name().unwrap_or("UTC").to_owned()
}

pub fn local(zone: &TimeZone, timestamp: Timestamp) -> String {
    timestamp
        .to_zoned(zone.clone())
        .strftime("%Y-%m-%d %H:%M %Z")
        .to_string()
}

pub fn time_left(seconds: i64) -> String {
    if seconds <= 0 {
        return "expired".to_owned();
    }
    let days = seconds / 86_400;
    let remainder = seconds % 86_400;
    let hours = remainder / 3_600;
    let minutes = (remainder % 3_600) / 60;
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

#[cfg(test)]
mod tests {
    use jiff::Timestamp;
    use serde_json::json;

    use jiff::tz::TimeZone;

    use super::{local, parse, resolve_zone, time_left, zone_label};

    #[test]
    fn utc_mode_never_consults_the_host_zone() {
        let (zone, warning) = resolve_zone(true);
        assert_eq!(zone, TimeZone::UTC);
        assert_eq!(warning, None);
        assert_eq!(zone_label(&zone), "UTC");
        let timestamp: Timestamp = "2026-07-31T20:18:56Z".parse().unwrap();
        assert_eq!(local(&zone, timestamp), "2026-07-31 20:18 UTC");
    }

    #[test]
    fn renders_a_named_zone_in_local_time() {
        let zone = TimeZone::get("Asia/Tokyo").expect("bundled or system tzdb");
        assert_eq!(zone_label(&zone), "Asia/Tokyo");
        let timestamp: Timestamp = "2026-07-31T20:18:56Z".parse().unwrap();
        assert_eq!(local(&zone, timestamp), "2026-08-01 05:18 JST");
    }

    #[test]
    fn parses_supported_timestamp_shapes() {
        let expected: Timestamp = "2026-07-27T00:00:00Z".parse().unwrap();
        assert_eq!(parse(&json!("2026-07-27T00:00:00Z")), Some(expected));
        assert_eq!(parse(&json!("2026-07-27T00:00:00")), Some(expected));
        assert_eq!(parse(&json!(1785110400)), Some(expected));
        assert_eq!(parse(&json!(1785110400000_i64)), Some(expected));
        assert_eq!(parse(&json!("1785110400000")), Some(expected));
        assert_eq!(parse(&json!("not-a-time")), None);
    }

    #[test]
    fn formats_countdowns() {
        assert_eq!(time_left(-1), "expired");
        assert_eq!(time_left(59), "0m");
        assert_eq!(time_left(3_661), "1h 1m");
        assert_eq!(time_left(90_000), "1d 1h");
    }
}
