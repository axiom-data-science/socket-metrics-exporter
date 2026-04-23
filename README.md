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

A Debian package (including a systemd unit) can be built with [`cargo-deb`](https://github.com/kornelski/cargo-deb).
The package links the binary statically against [musl](https://musl.libc.org/), so the
resulting `.deb` has no `libc6` dependency and works on bullseye, bookworm, trixie,
and recent Ubuntu releases:

    cargo install cargo-deb
    make deb

The resulting `.deb` is written to `target/x86_64-unknown-linux-musl/debian/`. Install with:

    sudo dpkg -i target/x86_64-unknown-linux-musl/debian/socket-metrics-exporter_*.deb

After install the `socket-metrics-exporter.service` unit is enabled and
started automatically. It reads environment overrides from
`/etc/default/socket-metrics-exporter` (see that file for the available
`SS_EXPORTER_*` variables).

HTTP feature set
----------------

To keep the binary small and buildable as a statically-linked musl artifact,
the `actix-web` dependency is configured with `default-features = false` and
only the `macros` feature enabled. This means the server does **not** include:

- TLS / HTTPS
- HTTP/2
- Response compression (gzip, brotli, zstd)
- Cookies

This is intentional for a Prometheus `/metrics` endpoint, which is normally
scraped over plain HTTP on a loopback interface (or fronted by a reverse
proxy such as nginx that handles TLS and compression). If you need any of
these features — e.g. to expose the exporter directly to the public internet
over HTTPS — re-enable the corresponding actix-web features in `Cargo.toml`,
and note that some (notably `compress-zstd`) require a C toolchain and will
break the musl-static build.
