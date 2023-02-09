test:
	bats ./tests/get-test-full-json.bats
	bats ./tests/get-time-test.bats
	bats ./tests/get-smartstream-test.bats
	bats ./tests/get-test.bats
	bats ./tests/get-test-json.bats
	bats ./tests/get-test-full.bats
	bats ./tests/post-test.bats


aarch64-linux: TARGET=aarch64-unknown-linux-musl
aarch64-linux: zigbuild

zigbuild:
	cargo zigbuild --target ${TARGET} -p http-source
