test-tests:
	cargo test
test-nightly_off-std_off:
	cd tests/test-nightly_off-std_off && cargo test
test-nightly_off-std_on:
	cd tests/test-nightly_off-std_on && cargo test
test-nightly_on-std_off:
	cd tests/test-nightly_on-std_off && cargo test
test-nightly_on-std_on:
	cd tests/test-nightly_on-std_on && cargo test

test: test-tests test-nightly_off-std_off test-nightly_off-std_on test-nightly_on-std_off test-nightly_on-std_on

.PHONY: test test-tests test-nightly_off-std_off test-nightly_off-std_on test-nightly_on-std_off test-nightly_on-std_on
