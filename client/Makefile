run-doc-tests:
	cargo test --doc

run-doc-tests-one-after-another:
	cargo test --doc -- --test-threads=1

run-unit-tests:
	cargo test --tests

run-unit-tests-one-after-another:
	cargo test --tests -- --test-threads=1

run-all-tests:
	${MAKE} run-doc-tests-one-after-another run-unit-tests-one-after-another

run-clippy:
	cargo clippy --all-targets -- -D warnings

run:
	cargo run