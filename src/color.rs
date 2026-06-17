//! Semantic terminal coloring and layout for human-readable output.
//!
//! Presentation only: it never computes a verdict, just words/colors/aligns
//! existing results. Colors are emitted only when stdout is a TTY and `NO_COLOR`
//! is unset, so piped or redirected output (including `--json` / `--csv`) stays
//! byte-stable. Column widths are measured on the uncolored text, so alignment
//! holds whether or not colors are on.

use std::io::IsTerminal;
use std::sync::OnceLock;

use reliakit_health::Health;

const ACCENT: &str = "38;2;88;166;255"; // azure — product identity
const MUTED: &str = "38;2;139;148;158"; // gray — field labels, detail
const GOOD: &str = "38;2;63;185;80"; // green — healthy / fast
const WARN: &str = "38;2;210;153;34"; // amber — degraded / slow
const BAD: &str = "38;2;248;81;73"; // red — failing / very slow

fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED
        .get_or_init(|| std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none())
}

fn paint(params: &str, text: &str) -> String {
    if enabled() {
        format!("\x1b[{params}m{text}\x1b[0m")
    } else {
        text.to_owned()
    }
}

/// A bold accent title line.
pub fn title(text: &str) -> String {
    paint(&format!("1;{ACCENT}"), text)
}

/// A bold section heading.
pub fn heading(text: &str) -> String {
    paint("1", text)
}

/// A muted field label.
pub fn label(text: &str) -> String {
    paint(MUTED, text)
}

fn latency_code(ms: u128) -> &'static str {
    if ms < 500 {
        GOOD
    } else if ms < 2_000 {
        WARN
    } else {
        BAD
    }
}

/// `"<ms> ms"`, colored by latency (green fast, amber slow, red very slow).
pub fn latency(ms: u128) -> String {
    paint(latency_code(ms), &format!("{ms} ms"))
}

/// The plain status word for a health status: `OK`, `DEGRADED`, or `DOWN`.
pub fn health_word_plain(status: Health) -> &'static str {
    match status {
        Health::Healthy => "OK",
        Health::Degraded => "DEGRADED",
        Health::Unhealthy => "DOWN",
    }
}

/// A health status as a bold colored word.
pub fn health_word(status: Health) -> String {
    let code = match status {
        Health::Healthy => GOOD,
        Health::Degraded => WARN,
        Health::Unhealthy => BAD,
    };
    paint(&format!("1;{code}"), health_word_plain(status))
}

/// A watch verdict: `steady` green, `alert` red.
pub fn verdict(word: &str) -> String {
    match word {
        "steady" => paint(GOOD, word),
        "alert" => paint(BAD, word),
        other => other.to_owned(),
    }
}

/// The hostname of a URL, dropping scheme, path, and any `user:pass@` credentials
/// so the endpoint can be shown without leaking a key.
pub fn host(url: &str) -> String {
    let after_scheme = url.split("://").nth(1).unwrap_or(url);
    let host_port = after_scheme.split('/').next().unwrap_or(after_scheme);
    host_port.rsplit('@').next().unwrap_or(host_port).to_owned()
}

/// One table cell: `plain` is measured for alignment, `display` is printed (it
/// may carry color codes).
pub struct Cell {
    plain: String,
    display: String,
}

impl Cell {
    /// A cell whose printed text is its measured text (no color).
    pub fn plain(text: impl Into<String>) -> Self {
        let plain = text.into();
        Cell {
            display: plain.clone(),
            plain,
        }
    }

    /// A cell measured by `plain` but printed as `display` (e.g. a colored label).
    pub fn styled(plain: impl Into<String>, display: impl Into<String>) -> Self {
        Cell {
            plain: plain.into(),
            display: display.into(),
        }
    }
}

/// Render rows of cells into left-aligned columns separated by `gap` spaces.
pub fn table(rows: &[Vec<Cell>], gap: usize) -> String {
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut widths = vec![0usize; columns];
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.plain.chars().count());
        }
    }

    let mut out = String::new();
    for row in rows {
        let mut line = String::new();
        for (i, cell) in row.iter().enumerate() {
            line.push_str(&cell.display);
            if i + 1 != row.len() {
                let pad = widths[i] - cell.plain.chars().count() + gap;
                line.push_str(&" ".repeat(pad));
            }
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latency_code_thresholds() {
        assert_eq!(latency_code(100), GOOD);
        assert_eq!(latency_code(800), WARN);
        assert_eq!(latency_code(3_000), BAD);
    }

    #[test]
    fn host_drops_scheme_path_and_credentials() {
        assert_eq!(
            host("https://user:pass@node.example.com/key=abc"),
            "node.example.com"
        );
        assert_eq!(host("http://localhost:8545"), "localhost:8545");
    }

    #[test]
    fn table_aligns_on_plain_width_ignoring_color() {
        // Colored first cells must still align by their uncolored width.
        let rows = vec![
            vec![Cell::styled("OK", paint(GOOD, "OK")), Cell::plain("up")],
            vec![
                Cell::styled("Latency", label("Latency")),
                Cell::plain("9 ms"),
            ],
        ];
        let rendered = table(&rows, 3);
        // "up" and "9 ms" start at the same column (after the 7-wide label + gap).
        let value_columns: Vec<usize> = rendered
            .lines()
            .map(|l| l.find("up").or_else(|| l.find("9 ms")).unwrap())
            .collect();
        assert_eq!(value_columns[0], value_columns[1]);
    }
}
