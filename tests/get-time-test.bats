#!/usr/bin/env bats

setup() {
    cargo build -p mock-http-server
    ./target/debug/mock-http-server & disown
    MOCK_PID=$!
    FILE=$(mktemp)
    cp ./tests/get-time-test-config.yaml $FILE
    UUID=$(uuidgen | awk '{print tolower($0)}')
    TOPIC=${UUID}-topic
    fluvio topic create $TOPIC

    sed -i.BAK "s/TOPIC/${TOPIC}/g" $FILE
    cat $FILE

    cargo build -p http-source
    ./target/debug/http-source --config $FILE  & disown
    CONNECTOR_PID=$!
}

teardown() {
    fluvio topic delete $TOPIC
    kill $MOCK_PID
    kill $CONNECTOR_PID
}

@test "http-connector-get-time-test" {
    MAX_MS_FOR_5_RECORDS=650
    echo "Starting consumer on topic $TOPIC"
    echo "This test ensures that with a http interval of 100ms, and 0ms source-linger time, 5 records should be produces in under ${MAX_MS_FOR_5_RECORDS}ms"
    sleep 13

    fluvio consume -T 5 -d $TOPIC | while read line; do
        difference=$((($(date +%s%N) - $line)/1000000))
        echo $difference
        [ $difference -lt ${MAX_MS_FOR_5_RECORDS} ]
    done
}

