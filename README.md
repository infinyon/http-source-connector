# Fluvio HTTP Inbound Connector

Read HTTP Responses given input HTTP request configuration options and produce them
to Fluvio topics.

This connector can be configured to operate in three modes.

- [Polling](#usage-example): Unless otherwise specified, the endpoint will be polled periodically, with the polling interval specified by providing the `interval` config option. Each response will be produced as an individual Fluvio record.
- [Streaming](#streaming-mode): When the `stream` config option is provided, the HTTP response will be processed as a [data stream](https://en.wikipedia.org/wiki/Chunked_transfer_encoding). A record will be produced to Fluvio every time a `delimiter` segment is encountered, which is set to `\n` by default.
- [WebSocket](#websocket-mode): When the provided `endpoint` config option is prefixed with `ws://`, a WebSocket connection will be established, and each incoming message will be produced.

Supports HTTP/1.0, HTTP/1.1, HTTP/2.0 protocols.

See [docs](https://www.fluvio.io/connectors/inbound/http/) here.
Tutorial for [HTTP to SQL Pipeline](https://www.fluvio.io/docs/tutorials/data-pipeline/).

### Configuration
| Option           | default                    | type            | description                                                                                |
|:-----------------|:---------------------------|:----------------|:-------------------------------------------------------------------------------------------|
| interval         | 10s                        | String          | Interval between each HTTP Request. This is in the form of "1s", "10ms", "1m", "1ns", etc. |
| method           | GET                        | String          | GET, POST, PUT, HEAD                                                                       |
| endpoint         | -                          | String          | HTTP URL endpoint. Use `ws://` for websocket URLs.                                         |
| headers          | -                          | Array\<String\> | Request header(s) "Key:Value" pairs                                                        |
| body             | -                          | String          | Request body e.g. in POST                                                                  |
| user-agent       | "fluvio/http-source 0.1.0" | String          | Request user-agent                                                                         |
| output_type      | text                       | String          | `text` = UTF-8 String Output, `json` = UTF-8 JSON Serialized String                        |
| output_parts     | body                       | String          | `body` = body only, `full` = all status, header and body parts                             |
| stream           | false                      | bool            | Flag to indicate HTTP streaming mode                                                       |
| delimiter        | '\n'                       | String          | Delimiter to separate records when producing from an HTTP streaming endpoint               |
| websocket_config | {}                         | Object          | WebSocket configuration object. See below.                                                 |

#### Record Type Output
| Matrix                                                      | Output                                  |
| :---------------------------------------------------------- | :-------------------------------------- |
| output_type = text (default), output_parts = body (default) | Only the body of the HTTP Response      |
| output_type = text (default), output_parts = full           | The full HTTP Response                  |
| output_type = json, output_parts = body (default)           | Only the "body" in JSON struct          |
| output_type = json, output_parts = full                     | HTTP "status", "body" and "header" JSON |

#### WebSocket Configuration
| Option                | default | type            | description                                                                                                                                                  |
|:----------------------|:--------|:----------------|:-------------------------------------------------------------------------------------------------------------------------------------------------------------|
| subscription_messages | []      | Array\<String\> | List of messages to send to the server after connection is established.                                                                                      |
| ping_interval_ms      | 10000   | int             | Interval in milliseconds to send ping messages to the server.                                                                                                |
| subscription_message  | -       | String          | (deprecated) Message to send to the server after connection is established. If provided with subscription_messages, subscription_message will be sent first. |

### Usage Example

This is an example of simple connector config file for polling an endpoint:

```yaml
# config-example.yaml
apiVersion: 0.1.0
meta:
  version: 0.4.3
  name: cat-facts
  type: http-source
  topic: cat-facts
  create-topic: true
  secrets:
    - name: AUTHORIZATION_TOKEN
http:
  endpoint: "https://catfact.ninja/fact"
  interval: 10s
  headers:
    - "Authorization: token ${{ secrets.AUTHORIZATION_TOKEN }}"
    - "Cache-Control: no-cache"
```

The produced record in Fluvio topic will be:
```json
{
  "fact": "The biggest wildcat today is the Siberian Tiger. It can be more than 12 feet (3.6 m) long (about the size of a small car) and weigh up to 700 pounds (317 kg).",
  "length": 158
}
```
### Secrets

Fluvio HTTP Source Connector supports Secrets in the `endpoint` and in the `headers` parameters:

```yaml
# config-example.yaml
apiVersion: 0.1.0
meta:
  version: 0.4.3
  name: cat-facts
  type: http-source
  topic: cat-facts
  create-topic: true
  secrets:
    - name: MY_SECRET_URL
    - name: MY_AUTHORIZATION_HEADER
http:
 endpoint:
   secret:
     name: MY_SECRET_URL
 headers:
  - "Authorization: ${{ secrets.MY_AUTHORIZATION_HEADER }}
 interval: 10s
```


### Transformations
Fluvio HTTP Source Connector supports [Transformations](https://www.fluvio.io/docs/concepts/transformations-chain/). Records can be modified before sending to Fluvio topic.

The previous example can be extended to add extra transformations to outgoing records:
```yaml
# config-example.yaml
apiVersion: 0.1.0
meta:
  version: 0.4.3
  name: cat-facts
  type: http-source
  topic: cat-facts
  create-topic: true
http:
  endpoint: "https://catfact.ninja/fact"
  interval: 10s
transforms:
  - uses: infinyon/jolt@0.1.0
    with:
      spec:
        - operation: default
          spec:
            source: "http-connector"
        - operation: remove
          spec:
            length: ""
```
In this case, additional transformation will be performed before records are sent to Fluvio topic: field `length` will be removed and
field `source` with string value `http-connector` will be added.

Now produced records will have a different shape, for example:
```json
{
  "fact": "A cat has more bones than a human; humans have 206, and the cat - 230.",
  "source": "http-connector"
}
```

Read more about [JSON to JSON transformations](https://www.fluvio.io/smartmodules/certified/jolt/).

### Streaming Mode

Provide the `stream` configuration option to enable streaming mode with `delimiter` to determine how the incoming records are separated.

```yaml
# config-example.yaml
apiVersion: 0.1.0
meta:
  version: 0.4.3
  name: wiki-updates
  type: http-source
  topic: wiki-updates
http:
  endpoint: "https://stream.wikimedia.org/v2/stream/recentchange"
  method: GET
  stream: true
  delimiter: "\n\n"
```

### Websocket Mode
Connect to a websocket endpoint using a `ws://` URL. When reading text messages, they are emitted as equivalent records. Binary messages are initially attempted to be converted into strings.

```yaml
# config-example.yaml
apiVersion: 0.1.0
meta:
  version: 0.4.3
  name: websocket-connector
  type: http-source
  topic: websocket-updates
http:
  endpoint: ws://websocket.example/websocket
```
