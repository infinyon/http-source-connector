#!/usr/bin/env bats

load './bats-helpers/bats-support/load'
load './bats-helpers/bats-assert/load'

setup() {
    cargo build -p mock-http-server
    ./target/debug/mock-http-server & disown
    MOCK_PID=$!
    FILE=$(mktemp)
    cp ./tests/get-test-full-config.yaml $FILE
    UUID=$(uuidgen | awk '{print tolower($0)}')
    TOPIC=${UUID}-topic
    fluvio topic create $TOPIC

    sed -i.BAK "s/TOPIC/${TOPIC}/g" $FILE
    cat $FILE

    cargo build -p http-source
    ./target/debug/http-source --config $FILE & disown
    CONNECTOR_PID=$!
}

teardown() {
    fluvio topic delete $TOPIC
    kill $MOCK_PID
    kill $CONNECTOR_PID
}

@test "http-connector-get-full-test" {
    count=1
    echo "Starting consumer on topic $TOPIC"
    sleep 10

    run fluvio consume --start 0 --end 0 -d $TOPIC
    assert_output --partial 'HTTP/1.1 200 OK'

    run fluvio consume --start 0 --end 0 -d $TOPIC
    assert_output --partial 'content-type: text/plain;charset=utf-8'

    run fluvio consume --start 1 --end 1 -d $TOPIC
    assert_output --partial 'Hello, Fluvio! - '
}

