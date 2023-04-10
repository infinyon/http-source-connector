#!/usr/bin/env bats

# setup() {
#    FILE=$(mktemp)
#    cp ./tests/post-test-config-v2.yaml $FILE
#    UUID=$(uuidgen | awk '{print tolower($0)}')
#    TOPIC=${UUID}-topic

#    fluvio cloud login --remote http://dev.infinyon.cloud
#    fluvio topic create $TOPIC
#    fluvio cloud connector create --config ./tests/cloud-http-post-test-v2.yaml

#    sed -i.BAK "s/TOPIC/${TOPIC}/g" $FILE
#    cat $FILE

#    cargo build -p http-source
#    ./target/debug/http-source --config $FILE & disown
#    CONNECTOR_PID=$!
#}

#teardown() {
#    fluvio topic delete $TOPIC
#    kill $CONNECTOR_PID
#}

@test "cloud-http-post-test-v2" {
  curl -X POST "https://httpbin.org/post" -H "accept: application/json" > out.http

  echo ./out.http
}
