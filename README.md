# URocket http stage

It proxy requests from http to an executable.

Openapi reference:
- https://swagger.io/specification/
- https://editor.swagger.io/

Scripting language are supported by specifying the executable

The openapi is used as it is, without any change, example:

```
paths:
  /pets:
    post:
      tags:
        - pet
      summary: Add a new pet to the store
      description: Add a new pet to the store
      operationId: addPet
      requestBody:
        description: Create a new pet in the store
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/Pet'
          application/xml:
            schema:
              $ref: '#/components/schemas/Pet'
          application/x-www-form-urlencoded:
            schema:
              $ref: '#/components/schemas/Pet'
        required: true
```

The service attach the backend callback using its own configuration file, `urocket-service.yaml`,
that replicate the `paths.uri.method` schema to define each `uri.method` callback infos:

```
paths:
  /get/pets:
    get:
      validatein: false
      inject:
        wd: /src/scripts/php
        env:
          - MYENV=CI
        cmd: /usr/bin/echo
        channel: "cmdline"
        encoding: json
      logstdout: true
      outtake: usocket://uri/{req_id}
      validate-out: false
```

This will execute `/usr/bin/echo` on work dir defined in `wd`, with env ... see below for the
details.

## Using the socket: php example

PHP use the socket for reply, i.e. libcurl:

```php
$ch = curl_init();

// $postdata = json_encode($data); // typically
$postdata = '{"handler": "pricechange", "pricelist": [{"articlenr":"12312", ...}]}';

curl_setopt($ch, CURLOPT_UNIX_SOCKET_PATH, "/tmp/uselessrock.sock");

curl_setopt($ch, CURLOPT_POST, 1);
curl_setopt($ch, CURLOPT_POSTFIELDS, $postdata);
curl_setopt($ch, CURLOPT_RETURNTRANSFER, 1); 
curl_setopt($ch, CURLOPT_HTTPHEADER, array('Content-Type: application/json'));

$result = curl_exec($ch);
curl_close($ch);
print_r ($result);
```


## Testing curl

Is a dummy hostname required?

> curl --unix-socket /var/run/docker.sock http://localhost/images/json

or:

> curl --unix-socket /var/run/docker.sock http:/images/json

see:
https://superuser.com/a/925610

> cURL 7.50 and up requires a valid URL to be provided, including a hostname, so to run the above examples with cURL 7.50, a "dummy" hostname has to be added

## Message in / Message out

A message type is identified by (path, verb), as defined in OpenAPI definition `paths.[path].[verb]`.
There are 4 distinct stage for a message type:

1. incoming: defined as http verb + payload
2. transported-in: defined as process execution env
3. transported-out: defined by ipc channel
4. outgoing: defined as http verb and payload

Stages 1. and 4. can add a layer of validation for payload,
incoming and/or outgoing, the validation is based on Open API definition.

For 2. and 3. : **both are a map between "http-path+verb"**:

```
paths:
  "get/pets":
    get:
      validate-in: false
      inject: {{ process-env }}
      logstdout: true | false
      outtake: usocket://one/the/uuuid-123123-ggess-2123123
      validate-out: false
    post: ...
      in: {{ process-env }}
      out: {{ ipc-channel }}
```

Note on **logstdout**: the service should be able to log stdout of the script.
This can be supported by specifying special header in incoming http request (http header),
or by other means, TBD.

### process-env

The process is started with these env variables settled:

```
URIPATH=/path/in/request/uri
REQUEST_ID={unique request id used to match the result}
```

```
wd: /path/to/wd
env: [string]
cmd: command line
channel: cmdline | stdin | ...
encoding: json
```

If "channel: cmdline" then payload is passed as escaped commandline argument, i.e.:

> [cmd] '{"my": "json", "payload": "et cetera", "et": true, "cetera": false}'

If "channel: stdin" then payload pass through the stdin

### outtake

example:

```
outtake: usocket://uri/{req_id}
```

it is a uri scheme, uscoket means the socketpath defined at top level,
`{req_id}` must be replaced  with the matching request id, ie.:

1. the process read ENV["REQUEST_ID] from environment
2. the process write reply payload in usocket://uri/$ENV["REQUEST_ID]

No others uri scheme are supported, service should refuse to start (if it starts with other schemes then it's a bug)


## TODO

It's a kind of plan.

This project is on POC stage. What is in **already**:

* there is an arbiter that store request_id and dispatch back response via a tokio channel
* frontserv::run_front() listen on port 8080 and use arbiter to store req_id
* backserv::run_backserv() listen on socket file and use arbiter to match and dispatch message to frontserv request waiting on channel.

**TODOs**:

- arbiter should create a real uuid

It is missing, an `RequestVisor` actor:

* a struct that accept configuration (defined in `urocket-service.yaml`) and to dispatch request
accordingly.
* has a static method `::new(conf, arbiter)`
* has a method `wait_for(req: Request<IncomingBody>) -> (rx: Receiver<ForHttpResponse>)`
* `wait_for` match the req.uri() and req.method() to execute the right action (an OS process)
* `wait_for` use the exitCode of the process to eventually retry or giving a failure response (i.e., http 500 code)
* `wait_for` create a new channel that will be returned to the caller, `(rv_channel_rx: Receiver<ForHttpResponse>, rv_channel_tx)`, and tokio::spawn an async that wait for the arbiter channel, or the exitCode, or the (optional) timeout. Based on which of those complete, tx.send() the payload (the payload can be the one received from the process or a placeholder created in case of timeout or error).
* (`wait_for` spawned) if the message comes from backend in-time, RequestVisor ask the Arbiter to remove the request, otherwise the request id and the rx_channel is moved to another "actor", collecting staff about it.
* `wait_for` return rv_channel_rx
* has a method `push_fulfill(req_id: String, ForHttpResponse)-> Result<bool,ErrorBack>` that return Ok(true) on success, ErrorBack when req_id is not matched, or anything else.
* `push_fulfill` match the req_id and send data over the right channel.
* if `push_fulfill` can not match the req_id, thus it return ErrorBack, the backserv use that for sending a feedback, over the unix socket, to the caller (for example the php script).
