#!/usr/bin/env bats

load './bats-helpers/bats-support/load'
load './bats-helpers/bats-assert/load'

setup() {
    FILE=$(mktemp)
    cp ./tests/cloud-http-get-header-test.yaml $FILE
    UUID=$(uuidgen | awk '{print tolower($0)}')
    TOPIC=${UUID}-topic
    CONNECTOR=${UUID}-get-header

    sed -i.BAK "s/CONNECTOR/${CONNECTOR}/g" $FILE
    sed -i.BAK "s/TOPIC/${TOPIC}/g" $FILE
    cat $FILE

    fluvio cloud login --email ${FLUVIO_CLOUD_TEST_USERNAME} --password ${FLUVIO_CLOUD_TEST_PASSWORD}
    fluvio topic create $TOPIC
    fluvio cloud connector create --config $FILE
}

teardown() {
    fluvio cloud connector delete $CONNECTOR
}

@test "cloud-http-get-header-test" {
    echo "Starting consumer on topic $TOPIC"
    echo "Using connector $CONNECTOR"
    sleep 25

    fluvio consume -B -d $TOPIC | jq '.body | fromjson | .headers["X-Custom-Value"]' | grep "AGoodCustomValue"
    assert_success
}
