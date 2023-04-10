test:
	bats ./tests/get-test-full-json.bats
	bats ./tests/get-time-test.bats
	bats ./tests/get-smartstream-test.bats
	bats ./tests/get-test.bats
	bats ./tests/get-test-json.bats
	bats ./tests/get-test-full.bats
	bats ./tests/post-test.bats
	bats ./tests/post-test-config-v2

test_fluvio_install:
	fluvio cluster delete
	fluvio cluster start --image-version latest
	sleep 10
	fluvio version
	fluvio topic list
	fluvio topic create foobar
	sleep 3
	echo foo | fluvio produce foobar
	fluvio consume foobar -B -d
