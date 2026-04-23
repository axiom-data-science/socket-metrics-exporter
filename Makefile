MUSL_TARGET := x86_64-unknown-linux-musl

.PHONY: all build release release-musl test fmt lint deb clean help

all: lint test build

help:
	@echo "Targets:"
	@echo "  build         cargo build --all-targets (debug)"
	@echo "  release       cargo build --release"
	@echo "  release-musl  cargo build --release --target $(MUSL_TARGET)"
	@echo "  test          cargo test"
	@echo "  fmt           apply rustfmt to all sources"
	@echo "  lint          check formatting (matches CI)"
	@echo "  deb           build a portable musl-static Debian package"
	@echo "  clean         cargo clean"

build:
	cargo build --all-targets

release:
	cargo build --release

release-musl:
	rustup target add $(MUSL_TARGET)
	cargo build --release --target $(MUSL_TARGET)

test:
	cargo test

fmt:
	cargo fmt --all

lint:
	cargo fmt --all -- --check

deb: release-musl
	cargo deb --no-build --target $(MUSL_TARGET)

clean:
	cargo clean
