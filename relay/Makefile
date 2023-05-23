compile-to-x86_64-unknown-linux-gnu:
	cross build --release --target x86_64-unknown-linux-gnu -v

build-release-mode:
	cargo build --release

run-doc-tests:
	cargo test --doc

run-unit-tests:
	cargo test --tests

run-all-tests: run-unit-tests run-doc-tests

run-clippy:
	cargo clippy --all-targets -- -D warnings

run:
	cargo run

run-release:
	cargo run --release