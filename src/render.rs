use serde::Serialize;

use crate::payload::{Credit, Summary};
use crate::timefmt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Tool {
    name: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonOutput<'a> {
    schema: &'static str,
    tool: Tool,
    source: &'static str,
    checked_at: String,
    checked_at_local: &'a str,
    time_zone: &'a str,
    next_expiry_seconds: Option<i64>,
    available_count: u64,
    total_earned_count: Option<u64>,
    credits: &'a [Credit],
    warnings: &'a [String],
}

pub fn json(summary: &Summary) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&JsonOutput {
        schema: "codex-reset-status/v1",
        tool: Tool {
            name: "codex-reset-status",
            version: env!("CARGO_PKG_VERSION"),
        },
        source: summary.source,
        checked_at: summary.checked_at.to_string(),
        checked_at_local: &summary.checked_at_local,
        time_zone: &summary.time_zone,
        next_expiry_seconds: summary.next_expiry_seconds(),
        available_count: summary.available_count,
        total_earned_count: summary.total_earned_count,
        credits: &summary.credits,
        warnings: &summary.warnings,
    })
}

pub fn text(summary: &Summary) -> String {
    let mut lines = vec![
        "Codex Reset Credits".to_owned(),
        format!(
            "Available: {} {}",
            summary.available_count,
            if summary.available_count == 1 {
                "reset"
            } else {
                "resets"
            }
        ),
    ];
    if let Some(total) = summary.total_earned_count {
        lines.push(format!("Total earned: {total}"));
    }
    if let Some(seconds) = summary.next_expiry_seconds() {
        lines.push(format!(
            "Use the first one within: {}",
            timefmt::time_left(seconds)
        ));
    }
    lines.push(format!(
        "Checked: {} ({})",
        summary.checked_at_local, summary.time_zone
    ));
    lines.push(String::new());

    if summary.credits.is_empty() {
        lines.push("No reset credits found.".to_owned());
    } else {
        // The zone is stated once in the "Checked" line, so the header stays
        // correct for both local and --utc output.
        let headers = ["#", "Status", "Type", "Use before", "Time left"];
        let rows: Vec<Vec<String>> = summary
            .credits
            .iter()
            .map(|credit| {
                vec![
                    credit.index.to_string(),
                    cell(credit.status.as_deref(), 16),
                    cell(credit.reset_type.as_deref(), 20),
                    cell(credit.expires_local.as_deref(), 24),
                    credit.time_left.clone(),
                ]
            })
            .collect();
        lines.push(table(&headers, &rows));
    }

    if !summary.warnings.is_empty() {
        lines.push(String::new());
        lines.push("Warnings:".to_owned());
        lines.extend(
            summary
                .warnings
                .iter()
                .map(|warning| format!("- {warning}")),
        );
    }
    lines.join("\n")
}

fn cell(value: Option<&str>, max_len: usize) -> String {
    let value = value
        .unwrap_or("-")
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>();
    if value.chars().count() <= max_len {
        value
    } else {
        format!("{}...", value.chars().take(max_len - 3).collect::<String>())
    }
}

fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let widths: Vec<usize> = headers
        .iter()
        .enumerate()
        .map(|(index, header)| {
            rows.iter()
                .map(|row| row[index].chars().count())
                .chain(std::iter::once(header.chars().count()))
                .max()
                .unwrap_or(header.len())
        })
        .collect();
    let format_row = |row: &[String]| {
        row.iter()
            .enumerate()
            .map(|(index, value)| format!("{value:<width$}", width = widths[index]))
            .collect::<Vec<_>>()
            .join("  ")
            .trim_end()
            .to_owned()
    };
    let header_row = headers
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<Vec<_>>();
    let separator = widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<_>>()
        .join("  ");
    std::iter::once(format_row(&header_row))
        .chain(std::iter::once(separator))
        .chain(rows.iter().map(|row| format_row(row)))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::cell;

    #[test]
    fn strips_terminal_control_characters() {
        assert_eq!(cell(Some("ok\u{1b}[31m"), 20), "ok [31m");
        assert_eq!(cell(Some("line\nnext"), 20), "line next");
    }
}
