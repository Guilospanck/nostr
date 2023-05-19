##! relay
relay-run:
	$(MAKE) -C relay/ run

relay-run-clippy:
	$(MAKE) -C relay/ run-clippy

relay-run-tests:
	$(MAKE) -C relay/ run-unit-tests

check-docker-engine-is-running:
	./check-docker-engine.sh

relay-compile-to-x86_64-unknown-linux-gnu: check-docker-engine-is-running
	$(MAKE) -C relay/ compile-to-x86_64-unknown-linux-gnu

##! client
client-run:
	$(MAKE) -C client/ run

client-run-clippy:
	$(MAKE) -C client/ run-clippy

client-run-tests:
	$(MAKE) -C client/ run-unit-tests

##! sdk
nostr-sdk-run-clippy:
	$(MAKE) -C nostr-sdk/ run-clippy

nostr-sdk-run-tests:
	$(MAKE) -C nostr-sdk/ run-unit-tests

##! All
run-all-clippy: relay-run-clippy client-run-clippy nostr-sdk-run-clippy

run-all-tests: relay-run-tests client-run-tests nostr-sdk-run-tests

