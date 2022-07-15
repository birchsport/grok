REL_TYPE = patch

.DEFAULT_GOAL := build

clean:
	cargo clean

build: src/*.rs
	cargo build

build_release: build
	cargo build --release

install: build
	cargo install --path . --root ~

release: build
	cargo release --execute --no-publish  ${REL_TYPE}
