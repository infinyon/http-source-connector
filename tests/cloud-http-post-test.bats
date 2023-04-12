#!/usr/bin/env bats

setup() {
    FILE=$(mktemp)
    cp ./tests/cloud-http-post-test.yaml $FILE
    UUID=$(uuidgen | awk '{print tolower($0)}')
    TOPIC=${UUID}-topic

    fluvio cloud login --email ${FLUVIO_CLOUD_TEST_USERNAME} --password ${FLUVIO_CLOUD_TEST_PASSWORD} --remote 'https://dev.infinyon.cloud'
    fluvio cloud cluster create || true
    fluvio topic create $TOPIC
    fluvio cloud connector create --config $FILE

    sed -i.BAK "s/TOPIC/${TOPIC}/g" $FILE
    cat $FILE

    cargo build -p http-source
    ./target/debug/http-source --config $FILE & disown
    CONNECTOR_PID=$!
}

teardown() {
    fluvio topic delete $TOPIC
    fluvio cloud connector delete cloud-http-post-test
    kill $CONNECTOR_PID
    fluvio cloud cluster delete ${FLUVIO_CLOUD_TEST_USERNAME}
}

@test "cloud-http-post-test" {
    count=1
    echo "Starting consumer on topic $TOPIC"
    sleep 13

    fluvio consume -B -d $TOPIC
    assert_output --partial "Peter Parker"
}
