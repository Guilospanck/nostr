run-doc-tests:
	cargo test --doc

run-doc-tests-one-after-another:
	cargo test --doc -- --test-threads=1

run-unit-tests:
	cargo test --tests

run-unit-tests-one-after-another:
	cargo test --tests -- --test-threads=1

run-all-tests:
	@if [ -n "$(SERIAL)" ]; then \
		${MAKE} run-doc-tests-one-after-another run-unit-tests-one-after-another; \
	else \
		${MAKE} run-doc-tests run-unit-tests; \
	fi

run-clippy:
	cargo clippy --all-targets -- -D warnings