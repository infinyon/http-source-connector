# Fluvio HTTP Inbound Connector

Read HTTP Responses given input HTTP request configuration options and `interval` x and produces them to Fluvio topics.

Supports HTTP/1.0, HTTP/1.1, HTTP/2.0 protocols.

See [docs](https://www.fluvio.io/connectors/inbound/http/) here.
Tutorial for [HTTP to SQL Pipeline](https://www.fluvio.io/docs/tutorials/data-pipeline/).

### Configuration
| Option       | default                    | type    | description                                                                                |
| :------------| :--------------------------| :-----  | :----------------------------------------------------------------------------------------- |
| interval     | 10s                        | String  | Interval between each HTTP Request. This is in the form of "1s", "10ms", "1m", "1ns", etc. |
| method       | GET                        | String  | GET, POST, PUT, HEAD                                                                       |
| endpoint     | -                          | String  | HTTP URL endpoint                                                                          |
| headers      | -                          | Array\<String\> | Request header(s) "Key:Value" pairs                                                  |
| body         | -                          | String  | Request body e.g. in POST                                                                  |
| user-agent   | "fluvio/http-source 0.1.0" | String  | Request user-agent                                                                         |
| output_type  | text                       | String  | `text` = UTF-8 String Output, `json` = UTF-8 JSON Serialized String                        |
| output_parts | body                       | String  | `body` = body only, `full` = all status, header and body parts                             |

#### Record Type Output
| Matrix                                                      | Output                                  |
| :---------------------------------------------------------- | :-------------------------------------- |
| output_type = text (default), output_parts = body (default) | Only the body of the HTTP Response      |
| output_type = text (default), output_parts = full           | The full HTTP Response                  |
| output_type = json, output_parts = body (default)           | Only the "body" in JSON struct          |
| output_type = json, output_parts = full                     | HTTP "status", "body" and "header" JSON |


### Usage Example

This is an example of simple connector config file:

```yaml
# config-example.yaml
apiVersion: 0.1.0
meta:
  version: 0.2.1
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
  version: 0.2.1
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
  version: 0.2.1
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
