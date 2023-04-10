#!/usr/bin/env bats

setup() {
    FILE=$(mktemp)
    cp ./tests/post-test-config-v2.yaml $FILE
    UUID=$(uuidgen | awk '{print tolower($0)}')
    TOPIC=${UUID}-topic

    fluvio cloud login --email ${FLUVIO_CLOUD_TEST_USERNAME} --password ${FLUVIO_CLOUD_TEST_PASSWORD} --remote 'https://dev.infinyon.cloud'
    fluvio topic create $TOPIC
    fluvio cloud connector create --config ./tests/cloud-http-post-test-v2.yaml

    sed -i.BAK "s/TOPIC/${TOPIC}/g" $FILE
    cat $FILE

    cargo build -p http-source
    ./target/debug/http-source --config $FILE & disown
    CONNECTOR_PID=$!
}

teardown() {
    fluvio topic delete $TOPIC
    fluvio cloud connector delete http-json-connector-v2
    kill $CONNECTOR_PID
}

@test "cloud-http-post-test-v2" {
    count=1
    echo "Starting consumer on topic $TOPIC"
    sleep 13

    fluvio consume -B -d $TOPIC | while read input; do
        expected="Hello, Pablo! - $count"
        echo $input = $expected
        [ "$input" = "$expected" ]
        count=$(($count + 1))
        if [ $count -eq 10 ]; then
            break;
        fi
    done
}
