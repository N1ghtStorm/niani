.PHONY: install build test clean

install:
	cargo install --path . --force

build:
	cargo build

test:
	cargo test

clean:
	cargo clean
