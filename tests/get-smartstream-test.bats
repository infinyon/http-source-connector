#!/usr/bin/env bats

setup() {
    cargo build -p mock-http-server
    ./target/debug/mock-http-server & disown
    MOCK_PID=$!
    FILE=$(mktemp)
    cp ./tests/get-smartstream-config.yaml $FILE
    UUID=$(uuidgen | awk '{print tolower($0)}')
    TOPIC=${UUID}-topic
    fluvio topic create $TOPIC

    MODULE=${UUID}-map
    cd ./crates/test-smartmodule-map
    smdk build
    smdk load
    cd - 

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
    fluvio smart-module delete $MODULE
    fluvio sm delete infinyon/map-uppercase@0.1.0
}

@test "http-connector-get-smartmodule test" {
    count=1
    echo "Starting consumer on topic $TOPIC"
    sleep 13

    fluvio consume -B -d $TOPIC | while read input; do
        expected="HELLO, FLUVIO! - $count"
        echo $input = $expected
        [ "$input" = "$expected" ]
        count=$(($count + 1))
        if [ $count -eq 10 ]; then
            break;
        fi
    done

}

