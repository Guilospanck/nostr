relay-run-clippy:
	$(MAKE) -C relay/ run-clippy

relay-run-tests:
	$(MAKE) -C relay/ run-unit-tests

check-docker-engine-is-running:
	./check-docker-engine.sh

relay-compile-to-x86_64-unknown-linux-gnu: check-docker-engine-is-running
	$(MAKE) -C relay/ compile-to-x86_64-unknown-linux-gnu
