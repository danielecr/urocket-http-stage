# URocket http stage

It proxy requests from http to an executable.

Openapi reference:
- https://swagger.io/specification/
- https://editor.swagger.io/

Scripting language are supported by specifying the executable

The openapi is used with `callbacks` in paths.path object:

```
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
      callbacks:
        cbusesocket:
          $ref: '#/components/schemas/cbusesocket'
```

The idea is to use `cbusesocket` to define a callback using unixsocket for reply
Eventually there is `cbdirectstream` to take standard output as a stream to be proxy.
Or even a `cbusepipe` to open a pipe with the script called.

the cbusesocket is defined in schemas as a regular object:

```
components:
  schemas:
    cbusesocket:
      type: object
      properties:
        socketpath:
          type: string
        pathname:
          type: string
          description: full path of the php script
        wd:
          type: string
          description: initial working directory
        env:
          type: array
	  items:
            type: string
            description: key = value string
        format:
          type: string
          description: json, xml, or parameters to pass to the php script
        callbackurl:
          type: string
          description: callback url complete of the unix socket, socket://url_to_cb/
        callback_verb:
          type: string
          enum:
            - get
            - post
            - put
```

From the point of view of the called script, some parameters are accessible in the environment:
format, callbackurl, callback_verb, orig_url_path

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


