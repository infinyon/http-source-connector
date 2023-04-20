#!/usr/bin/env bats

load './bats-helpers/bats-support/load'
load './bats-helpers/bats-assert/load'

setup() {
    FILE=$(mktemp)
    cp ./tests/cloud-http-get-test.yaml $FILE
    UUID=$(uuidgen | awk '{print tolower($0)}')
    TOPIC=${UUID}-topic
    CONNECTOR=${UUID}-get
    VERSION=$(cat ./crates/http-source/hub/package-meta.yaml | grep "^version:" | cut -d" " -f2)

    sed -i.BAK "s/CONNECTOR/${CONNECTOR}/g" $FILE
    sed -i.BAK "s/TOPIC/${TOPIC}/g" $FILE
    sed -i.BAK "s/VERSION/${VERSION}/g" $FILE
    cat $FILE

    fluvio cloud login --email ${FLUVIO_CLOUD_TEST_USERNAME} --password ${FLUVIO_CLOUD_TEST_PASSWORD}
    fluvio topic create $TOPIC
    fluvio cloud connector create --config $FILE
}

teardown() {
    fluvio cloud connector delete $CONNECTOR
}

@test "cloud-http-get-test" {
    echo "Starting consumer on topic $TOPIC"
    echo "Using connector $CONNECTOR"
    sleep 45

    echo "Pre-check Connectors Statuses"
    fluvio cloud connector list

    echo "Initializing periodic status check"
    for i in {0..6}
    do
        if fluvio cloud connector list | sed 1d | grep "$CONNECTOR" | grep "Running" ; then
            echo "Connector $CONNECTOR is already Running!"
            break
        else
            echo "Attempt $i, not Running yet. Retrying after sleep"
            sleep 30
        fi
    done

    echo "Check connector logs"
    fluvio cloud connector logs $CONNECTOR || true

    echo "Check connector is status before testing"
    fluvio cloud connector list

    fluvio consume -B -d $TOPIC | jq .status.code | grep 200
    assert_success
}
