test:
	bats ./tests/get-test.bats
	bats ./tests/get-test-json.bats
	bats ./tests/get-test-full.bats
	bats ./tests/get-test-full-json.bats
	bats ./tests/get-stream-test.bats
	bats ./tests/get-stream-test-json.bats
	bats ./tests/get-stream-test-full.bats
	bats ./tests/get-stream-test-full-json.bats
	bats ./tests/post-test.bats
	bats ./tests/get-time-test.bats
	bats ./tests/get-smartmodule-test.bats

cloud_e2e_test:
	bats ./tests/cloud-http-get-test.bats
	bats ./tests/cloud-http-post-test.bats
	bats ./tests/cloud-http-get-header-test.bats

test_fluvio_install:
	fluvio version
	fluvio topic list
	fluvio topic create foobar
	sleep 3
	echo foo | fluvio produce foobar
	fluvio consume foobar -B -d
