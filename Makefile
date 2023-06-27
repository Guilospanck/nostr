##! relay
relay-run:
	$(MAKE) -C crates/relay/ run

relay-run-clippy:
	$(MAKE) -C crates/relay/ run-clippy

relay-run-tests:
	$(MAKE) -C crates/relay/ run-all-tests

check-docker-engine-is-running:
	./check-docker-engine.sh

relay-compile-to-x86_64-unknown-linux-gnu: check-docker-engine-is-running
	$(MAKE) -C crates/relay/ compile-to-x86_64-unknown-linux-gnu

##! client
client-run:
	$(MAKE) -C crates/client/ run

client-run-clippy:
	$(MAKE) -C crates/client/ run-clippy

client-run-tests:
	$(MAKE) -C crates/client/ run-all-tests

##! sdk
nostr-sdk-run-clippy:
	$(MAKE) -C crates/nostr-sdk/ run-clippy

nostr-sdk-run-tests:
	$(MAKE) -C crates/nostr-sdk/ run-all-tests

##! All
run-all-clippy:
	cargo clippy --all-targets -- -D warnings

run-all-check:
	cargo check --all-targets

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

coverage-and-open: coverage
	open -a Google\ Chrome.app target/debug/coverage/index.html

##! Generate docs for nostr-sdk
generate-docs:
	cargo doc --open --no-deps --package nostr-sdk
