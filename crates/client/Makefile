run-unit-tests:
	cargo test --tests

run-unit-tests-one-after-another:
	cargo test --tests -- --test-threads=1

run-all-tests:
	@if [ -n "$(SERIAL)" ]; then \
		${MAKE} run-unit-tests-one-after-another; \
	else \
		${MAKE} run-unit-tests; \
	fi

run-clippy:
	cargo clippy --all-targets -- -D warnings

run:
	cargo run