//! Semantic terminal coloring for human-readable output.
//!
//! Colors are emitted only when stdout is a TTY and `NO_COLOR` is unset, so
//! piped or redirected output (including `--json` and `--csv`) stays byte-stable.

use std::io::IsTerminal;
use std::sync::OnceLock;

use reliakit_health::Health;

fn colors_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED
        .get_or_init(|| std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none())
}

fn paint(code: &str, text: &str) -> String {
    if colors_enabled() {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_owned()
    }
}

/// ANSI code for a latency value: green when fast, yellow when slow, red when
/// very slow (same thresholds as the health classifier).
fn latency_code(ms: u128) -> &'static str {
    if ms < 500 {
        "32"
    } else if ms < 2_000 {
        "33"
    } else {
        "31"
    }
}

/// `"<ms> ms"`, colored by latency.
pub fn latency(ms: u128) -> String {
    paint(latency_code(ms), &format!("{ms} ms"))
}

/// A health status label, colored by severity.
pub fn health_label(status: Health) -> String {
    let code = match status {
        Health::Healthy => "32",
        Health::Degraded => "33",
        Health::Unhealthy => "31",
    };
    paint(code, status.as_str())
}

/// A watch verdict label: `steady` green, `alert` red.
pub fn verdict(label: &str) -> String {
    match label {
        "steady" => paint("32", label),
        "alert" => paint("31", label),
        other => other.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latency_code_thresholds() {
        assert_eq!(latency_code(100), "32");
        assert_eq!(latency_code(800), "33");
        assert_eq!(latency_code(3_000), "31");
    }
}
