[![CI](https://github.com/axiom-data-science/socket-metrics-exporter/actions/workflows/ci.yml/badge.svg)](https://github.com/axiom-data-science/socket-metrics-exporter/actions/workflows/ci.yml)
[![Security audit](https://github.com/axiom-data-science/socket-metrics-exporter/actions/workflows/audit.yml/badge.svg)](https://github.com/axiom-data-science/socket-metrics-exporter/actions/workflows/audit.yml)


socket-metrics-exporter
===============

Prometheus metrics exporter for socket metrics data

Copyright 2026 Axiom Data Science, LLC

See LICENSE for details.

Building
--------

In order to build the project, contributors need rust, see
[Install Rust](https://www.rust-lang.org/tools/install) for details about
installing the rust development environment on your system.

To build the project:

    cargo build

To run the binary without building a release version or installing to a locally available path:

    cargo run

For details about `cargo` and using `cargo`, please see [The Cargo Book](https://doc.rust-lang.org/cargo/commands/index.html)

Debian package
--------------

A Debian package (including a systemd unit) can be built with [`cargo-deb`](https://github.com/kornelski/cargo-deb):

    cargo install cargo-deb
    cargo deb

The resulting `.deb` is written to `target/debian/`. Install it with:

    sudo dpkg -i target/debian/socket-metrics-exporter_*.deb

After install the `socket-metrics-exporter.service` unit is enabled and
started automatically. It reads environment overrides from
`/etc/default/socket-metrics-exporter` (see that file for the available
`SS_EXPORTER_*` variables).
