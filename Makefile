.PHONY: all build release test fmt lint deb clean help

all: lint test build

help:
	@echo "Targets:"
	@echo "  build    cargo build --all-targets (debug)"
	@echo "  release  cargo build --release"
	@echo "  test     cargo test"
	@echo "  fmt      apply rustfmt to all sources"
	@echo "  lint     check formatting (matches CI)"
	@echo "  deb      build the Debian package via cargo-deb"
	@echo "  clean    cargo clean"

build:
	cargo build --all-targets

release:
	cargo build --release

test:
	cargo test

fmt:
	cargo fmt --all

lint:
	cargo fmt --all -- --check

deb:
	cargo deb

clean:
	cargo clean
