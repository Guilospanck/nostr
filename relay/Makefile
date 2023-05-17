compile-to-x86_64-unknown-linux-gnu:
	cross build --release --target x86_64-unknown-linux-gnu -v

run-unit-tests:
	cargo test --tests

run-clippy:
	cargo clippy --all-targets -- -D warnings