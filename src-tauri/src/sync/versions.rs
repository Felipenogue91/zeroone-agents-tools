//! Installed CLI versions vs the versions the sync formats were tested with.
//! Config formats of Codex and agy change fast: an untested version is
//! reported so the user checks the diff preview before applying.

use super::model::Cli;
use regex::Regex;
use serde::Serialize;
use std::sync::mpsc;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliVersion {
    pub cli: Cli,
    pub installed: Option<String>,
    /// Tested range, human readable.
    pub tested: String,
    /// Installed and inside the tested range.
    pub known: bool,
    pub message: Option<String>,
}

/// (cli, binary, min inclusive, max exclusive) as (major, minor).
const TESTED: [(Cli, &str, (u64, u64), (u64, u64)); 3] = [
    (Cli::Claude, "claude", (2, 0), (3, 0)),
    (Cli::Codex, "codex", (0, 160), (0, 180)),
    (Cli::Agy, "agy", (1, 5), (1, 6)),
];

pub fn parse_version(out: &str) -> Option<String> {
    Regex::new(r"\d+\.\d+(\.\d+)?")
        .unwrap()
        .find(out)
        .map(|m| m.as_str().to_string())
}

fn major_minor(v: &str) -> Option<(u64, u64)> {
    let mut it = v.split('.');
    Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?))
}

fn range_label(min: (u64, u64), max: (u64, u64)) -> String {
    if min.0 == max.0 && max.1 == min.1 + 1 {
        format!("{}.{}.x", min.0, min.1)
    } else if max.1 == 0 && max.0 == min.0 + 1 {
        format!("{}.x", min.0)
    } else {
        format!("{}.{} a {}.{}", min.0, min.1, max.0, max.1 - 1)
    }
}

pub fn classify(cli: Cli, installed: Option<String>) -> CliVersion {
    let (_, _, min, max) = TESTED.iter().find(|t| t.0 == cli).copied().unwrap();
    let tested = range_label(min, max);
    let known = installed
        .as_deref()
        .and_then(major_minor)
        .map(|mm| mm >= min && mm < max)
        .unwrap_or(false);
    let message = match (&installed, known) {
        (None, _) => None,
        (Some(_), true) => None,
        (Some(v), false) => Some(format!(
            "{} {} não foi testado com este app (testado: {}); confira a prévia antes de aplicar.",
            cli.label(),
            v,
            tested
        )),
    };
    CliVersion {
        cli,
        installed,
        tested,
        known,
        message,
    }
}

/// `<bin> --version` with a timeout (a hung CLI must not hang the app).
fn run_version(bin: &str) -> Option<String> {
    let (tx, rx) = mpsc::channel();
    let bin = bin.to_string();
    std::thread::spawn(move || {
        let out = std::process::Command::new(&bin).arg("--version").output();
        let _ = tx.send(out);
    });
    let out = rx.recv_timeout(Duration::from_secs(5)).ok()?.ok()?;
    if !out.status.success() {
        return None;
    }
    parse_version(&String::from_utf8_lossy(&out.stdout))
}

pub fn detect_all() -> Vec<CliVersion> {
    TESTED
        .iter()
        .map(|(cli, bin, _, _)| classify(*cli, run_version(bin)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cli_outputs() {
        assert_eq!(
            parse_version("2.1.331 (Claude Code)").as_deref(),
            Some("2.1.331")
        );
        assert_eq!(
            parse_version("codex-cli 0.171.0").as_deref(),
            Some("0.171.0")
        );
        assert_eq!(parse_version("1.5.16\n").as_deref(), Some("1.5.16"));
        assert_eq!(parse_version("nope"), None);
    }

    #[test]
    fn classifies_ranges() {
        assert!(classify(Cli::Codex, Some("0.171.0".into())).known);
        let newer = classify(Cli::Codex, Some("0.190.2".into()));
        assert!(!newer.known);
        assert!(newer.message.unwrap().contains("0.160 a 0.179"));
        assert!(classify(Cli::Agy, Some("1.5.16".into())).known);
        assert!(!classify(Cli::Agy, Some("1.6.0".into())).known);
        assert!(classify(Cli::Claude, Some("2.1.331".into())).known);
        let missing = classify(Cli::Claude, None);
        assert!(!missing.known && missing.message.is_none());
    }
}
