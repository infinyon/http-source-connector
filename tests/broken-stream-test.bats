#!/usr/bin/env bats

load './bats-helpers/bats-assert/load'
load './bats-helpers/bats-support/load'

setup() {
    cargo build -p mock-http-server
    start_mock_server

    CONFIG_FILE=$(mktemp)
    cp ./tests/get-stream-test-config.yaml $CONFIG_FILE
    UUID=$(uuidgen | awk '{print tolower($0)}')
    TOPIC=${UUID}-topic
    fluvio topic create $TOPIC

    sed -i.BAK "s/TOPIC/${TOPIC}/g" $CONFIG_FILE
    cat $CONFIG_FILE

    cargo build -p http-source
    ./target/debug/http-source --config $CONFIG_FILE & disown
    CONNECTOR_PID=$!
}

start_mock_server() {
    ./target/debug/mock-http-server & disown
    MOCK_PID=$!
}

teardown() {
    echo "topic"
    echo $TOPIC 
    fluvio topic delete $TOPIC
    kill $MOCK_PID
    kill $CONNECTOR_PID
}

# A SIGKILL sent to the server will result in a EOF being sent to the connector,
# which results in a reqwest error
@test "http-connector-broken-stream-test" {
    echo "Starting consumer on topic $TOPIC"
    sleep 3

    curl -s http://localhost:8080/get
    sleep 1

    run fluvio consume --start 0 --end 0 -d $TOPIC 
    assert_output --partial $'event:get request(s)\ndata:{ \"gets\": 1, \"posts\": 0 }'

    kill $MOCK_PID
    start_mock_server
    sleep 1

    curl -s http://localhost:8080/get
    sleep 1

    run fluvio consume --start 1 --end 1 -d $TOPIC 
    assert_output --partial $'event:get request(s)\ndata:{ \"gets\": 1, \"posts\": 0 }'


}