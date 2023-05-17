relay-run-clippy:
	$(MAKE) -C relay/ run-clippy

relay-run-tests:
	$(MAKE) -C relay/ run-unit-tests

relay-compile-to-x86_64-unknown-linux-gnu:
	$(MAKE) -C relay/ compile-to-x86_64-unknown-linux-gnu
