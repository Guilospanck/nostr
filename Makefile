##! relay
relay-run:
	$(MAKE) -C relay/ run

relay-run-clippy:
	$(MAKE) -C relay/ run-clippy

relay-run-tests:
	$(MAKE) -C relay/ run-all-tests

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
	$(MAKE) -C client/ run-all-tests

##! sdk
nostr-sdk-run-clippy:
	$(MAKE) -C nostr-sdk/ run-clippy

nostr-sdk-run-tests:
	$(MAKE) -C nostr-sdk/ run-all-tests

##! All
run-all-clippy: relay-run-clippy client-run-clippy nostr-sdk-run-clippy

##! If you wanna run them in a serial way, pass SERIAL=true to the command. Ex.: make run-all-tests SERIAL=true
run-all-tests: relay-run-tests client-run-tests nostr-sdk-run-tests

##! Only in dev
relay-upload-compiled-to-server:
	$(MAKE) -C deploy/ relay-upload-compiled-to-server

##! Tag and push it. Example: ❯ make tag-and-push new_tag=v0.0.2
tag-and-push:
	git tag -a ${new_tag} && git push origin ${new_tag}

##! Generate new test coverage
coverage:
	./generate_coverage.sh
