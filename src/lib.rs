//! Parse the output of `ss -s` into structured socket statistics.
//!
//! Example `ss -s` output:
//!
//! ```text
//! Total: 1740
//! TCP:   457 (estab 205, closed 162, orphaned 0, timewait 1)
//!
//! Transport Total     IP        IPv6
//! RAW      0         0         0
//! UDP      18        17        1
//! TCP      295       285       10
//! INET     313       302       11
//! FRAG     0         0         0
//! ```

use std::collections::HashMap;
use std::process::Stdio;

use tokio::process::Command;

/// Errors produced when collecting socket statistics.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed to execute `ss -s`: {0}")]
    Spawn(#[from] std::io::Error),

    #[error("`ss -s` exited with non-zero status {status}: {stderr}")]
    NonZeroExit { status: i32, stderr: String },

    #[error("failed to parse `ss -s` output: {0}")]
    Parse(String),
}

/// TCP socket counts broken down by connection state, as reported on the
/// `TCP:` line of `ss -s`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TcpStats {
    /// Total TCP sockets (the number preceding the parenthesised breakdown).
    pub total: u64,
    /// Per-state counts. Keys are the labels used by `ss` (e.g. `estab`,
    /// `closed`, `orphaned`, `timewait`, `synrecv`).
    pub states: HashMap<String, u64>,
}

/// A row of the `Transport` table. Each transport (RAW, UDP, TCP, INET, FRAG)
/// has a total and per-address-family breakdown.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TransportRow {
    pub total: u64,
    pub ipv4: u64,
    pub ipv6: u64,
}

/// Parsed `ss -s` output.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SocketStats {
    /// `Total:` line — total sockets across all transports.
    pub total: u64,
    /// `TCP:` line — TCP totals and state breakdown.
    pub tcp: TcpStats,
    /// Transport table rows keyed by transport name (e.g. `RAW`, `UDP`).
    pub transports: HashMap<String, TransportRow>,
}

/// Run `ss -s` and return the parsed statistics.
pub async fn collect() -> Result<SocketStats, Error> {
    let output = Command::new("ss")
        .arg("-s")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !output.status.success() {
        return Err(Error::NonZeroExit {
            status: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse(&stdout)
}

/// Parse the textual output of `ss -s`.
pub fn parse(input: &str) -> Result<SocketStats, Error> {
    let mut stats = SocketStats::default();
    let mut in_transport_table = false;

    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("Total:") {
            stats.total = parse_u64(rest.trim())?;
        } else if let Some(rest) = line.strip_prefix("TCP:") {
            stats.tcp = parse_tcp_line(rest.trim())?;
        } else if line.starts_with("Transport") {
            in_transport_table = true;
        } else if in_transport_table {
            let (name, row) = parse_transport_row(line)?;
            stats.transports.insert(name, row);
        }
    }

    Ok(stats)
}

fn parse_tcp_line(rest: &str) -> Result<TcpStats, Error> {
    // Shape: `457 (estab 205, closed 162, orphaned 0, timewait 1)`
    let (total_str, breakdown) = match rest.split_once('(') {
        Some((a, b)) => (a.trim(), Some(b.trim_end_matches(')'))),
        None => (rest, None),
    };
    let total = parse_u64(total_str)?;

    let mut states = HashMap::new();
    if let Some(body) = breakdown {
        for part in body.split(',') {
            let mut it = part.split_whitespace();
            let name = it
                .next()
                .ok_or_else(|| Error::Parse(format!("missing state name in {part:?}")))?;
            let count = it
                .next()
                .ok_or_else(|| Error::Parse(format!("missing state count in {part:?}")))?;
            states.insert(name.to_string(), parse_u64(count)?);
        }
    }
    Ok(TcpStats { total, states })
}

fn parse_transport_row(line: &str) -> Result<(String, TransportRow), Error> {
    // Shape: `RAW      0         0         0`
    let mut it = line.split_whitespace();
    let name = it
        .next()
        .ok_or_else(|| Error::Parse(format!("empty transport row: {line:?}")))?;
    let total = parse_u64(
        it.next()
            .ok_or_else(|| Error::Parse(format!("transport row missing total: {line:?}")))?,
    )?;
    let ipv4 = parse_u64(
        it.next()
            .ok_or_else(|| Error::Parse(format!("transport row missing ipv4: {line:?}")))?,
    )?;
    let ipv6 = parse_u64(
        it.next()
            .ok_or_else(|| Error::Parse(format!("transport row missing ipv6: {line:?}")))?,
    )?;
    Ok((name.to_string(), TransportRow { total, ipv4, ipv6 }))
}

fn parse_u64(s: &str) -> Result<u64, Error> {
    s.parse()
        .map_err(|e| Error::Parse(format!("expected integer, got {s:?}: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "Total: 1740
TCP:   457 (estab 205, closed 162, orphaned 0, timewait 1)

Transport Total     IP        IPv6
RAW\t  0         0         0
UDP\t  18        17        1
TCP\t  295       285       10
INET\t  313       302       11
FRAG\t  0         0         0
";

    #[test]
    fn parses_total() {
        let s = parse(SAMPLE).unwrap();
        assert_eq!(s.total, 1740);
    }

    #[test]
    fn parses_tcp_states() {
        let s = parse(SAMPLE).unwrap();
        assert_eq!(s.tcp.total, 457);
        assert_eq!(s.tcp.states.get("estab"), Some(&205));
        assert_eq!(s.tcp.states.get("closed"), Some(&162));
        assert_eq!(s.tcp.states.get("orphaned"), Some(&0));
        assert_eq!(s.tcp.states.get("timewait"), Some(&1));
    }

    #[test]
    fn parses_transport_table() {
        let s = parse(SAMPLE).unwrap();
        assert_eq!(
            s.transports.get("UDP"),
            Some(&TransportRow {
                total: 18,
                ipv4: 17,
                ipv6: 1,
            })
        );
        assert_eq!(
            s.transports.get("TCP"),
            Some(&TransportRow {
                total: 295,
                ipv4: 285,
                ipv6: 10,
            })
        );
        assert_eq!(
            s.transports.get("INET"),
            Some(&TransportRow {
                total: 313,
                ipv4: 302,
                ipv6: 11,
            })
        );
        assert_eq!(
            s.transports.get("FRAG"),
            Some(&TransportRow {
                total: 0,
                ipv4: 0,
                ipv6: 0,
            })
        );
        assert_eq!(
            s.transports.get("RAW"),
            Some(&TransportRow {
                total: 0,
                ipv4: 0,
                ipv6: 0,
            })
        );
    }

    #[test]
    fn tcp_line_without_breakdown() {
        let stats = parse_tcp_line("42").unwrap();
        assert_eq!(stats.total, 42);
        assert!(stats.states.is_empty());
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse("TCP:   not-a-number").is_err());
    }
}
