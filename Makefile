.PHONY: build build-api build-desktop test lint clean

build:
	cd core && cargo build
	cd api && cargo build

build-api:
	cd api && cargo build

build-desktop:
	cd desktop/src-tauri && cargo build

test:
	cd core && cargo test
	cd api && cargo test

lint:
	cd core && cargo clippy -- -D warnings && cargo fmt --check
	cd api && cargo clippy -- -D warnings && cargo fmt --check

clean:
	cd core && cargo clean
	cd api && cargo clean
	cd desktop/src-tauri && cargo clean
