## Common CRUD Endpoints

The following CRUD endopints are implemented for

- [`connection-definitions`](#v1connection-definitions-connection-definitions)
- [`connection-model-definitions`](#v1connection-model-definitions-connection-model-definitions)
- [`connection-model-schemas`](#v1connection-model-schemas-connection-model-schemas)
- [`connection-oauth-definitions`](#v1connection-oauth-definitions-connection-oauth-definitions)
- [`common-models`](#v1common-models-common-models)

### `GET` Requests

All `GET` requests return a list of models filtered by the query parameters. The query parameters can be of any field but must match the field value exactly. There are also 2 special query paramters, `limit` and `skip`. These can be used for pagination since they will `limit` the amount of records returned and `skip` records before returning.

All `GET` responses will be the following format:

```
{
  "rows": [
    {
      <model1>
    },
    {
      <model2>
    }
  ],
  "total": 2,
  "skip": 0,
  "limit": 20
}
```

### `POST` Requests

All `POST` requests will create a record of that model type. The `body` of the `POST` request must be in the shape of the model type, except without the `_id` field and metadata fields. The exact `body` payloads for each model are listed below. On success, the model will be returned along with the newly created `_id` and metadata fields.

### `PATCH` Requests

All `PATCH` requests will update a record based on the `:id` passed in the endpoint path which is `/v1/<endpoint path>/:id`. The `body` of the request is identical to the `body` in the request for `POST` endpoints. On success, the response will be:

```json
{
  "success": true
}
```

### `DELETE` Requests

All `DELETE` requests will delete a record based on the `:id` passed in the endpoint path which is `/v1/<endpoint path>/:id`. On success, the newly deleted model will be returned.


### `/v1/connection-oauth-definitions` Connection OAuth Definitions

### `POST` Requests

The `POST` request has the format:

```json
{
    "connectionPlatform": "xero",
    "platformRedirectUri": "https://login.xero.com/identity/connect/authorize?response_type=code",
    "iosRedirectUri": "/connection-oauth/callback",
    "scopes": "",
    "init": {
        "configuration": {
            "baseUrl": "https://identity.xero.com/",
            "path": "connect/token",
            "authMethod": {
                "type": "None"
            },
            "headers": {
                "connection": [
                    "keep-alive"
                ],
                "accept": [
                    "application/json;charset=utf-8"
                ],
                "authorization": [
                    "{{ authorization }}"
                ]
            },
            "schemas": {},
            "samples": {},
            "responses": [],
            "content": "form"
        },
        "compute": "function btoa(str) { \n  const base64Chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/'; \n\n  let result = ''; \n  let i = 0; \n\n  while (i < str.length) { \n    const byte1 = str.charCodeAt(i++); \n    const byte2 = i < str.length ? str.charCodeAt(i++) : 0; \n    const byte3 = i < str.length ? str.charCodeAt(i++) : 0; \n\n    const triplet = (byte1 << 16) | (byte2 << 8) | byte3; \n\n    const char1 = (triplet >> 18) & 0x3F; \n    const char2 = (triplet >> 12) & 0x3F; \n    const char3 = (triplet >> 6) & 0x3F; \n    const char4 = triplet & 0x3F; \n\n    result += base64Chars.charAt(char1) + base64Chars.charAt(char2) +\n(i < str.length + 2 ? base64Chars.charAt(char3) : '=') +\n(i < str.length + 1 ? base64Chars.charAt(char4) : '='); \n } \n\n  return result; \n } \n\nfunction compute(payload) { \n  const credentials = payload.clientId + \":\" + payload.clientSecret;\n  const encodedCredentials = btoa(credentials);\n  return \"Basic \" + encodedCredentials;\n}; function headers(payload) { const credentials = payload.clientId + \":\" + payload.clientSecret; const encodedCredentials = btoa(credentials); return { authorization: \"Basic \" + encodedCredentials }; }; function body(payload) { const body = {grant_type: \"authorization_code\", code: payload.metadata.code, redirect_uri: payload.metadata.redirectUri}; return body; }; function compute(payload) { return { headers: headers(payload), body: body(payload) }; };",
        "responseCompute": "function compute(payload) { return { accessToken: payload.access_token, refreshToken: payload.refresh_token, expiresIn: payload.expires_in, tokenType: payload.token_type }; }"
    },
    "refresh": {
        "configuration": {
            "baseUrl": "https://identity.xero.com/",
            "path": "connect/token",
            "authMethod": {
                "type": "None"
            },
            "headers": {
                "connection": [
                    "keep-alive"
                ],
                "accept": [
                    "application/json;charset=utf-8"
                ],
                "authorization": [
                    "{{ authorization }}"
                ]
            },
            "schemas": {},
            "samples": {},
            "responses": [],
            "content": "form"
        },
        "compute": "function btoa(str) { \n  const base64Chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/'; \n\n  let result = ''; \n  let i = 0; \n\n  while (i < str.length) { \n    const byte1 = str.charCodeAt(i++); \n    const byte2 = i < str.length ? str.charCodeAt(i++) : 0; \n    const byte3 = i < str.length ? str.charCodeAt(i++) : 0; \n\n    const triplet = (byte1 << 16) | (byte2 << 8) | byte3; \n\n    const char1 = (triplet >> 18) & 0x3F; \n    const char2 = (triplet >> 12) & 0x3F; \n    const char3 = (triplet >> 6) & 0x3F; \n    const char4 = triplet & 0x3F; \n\n    result += base64Chars.charAt(char1) + base64Chars.charAt(char2) +\n(i < str.length + 2 ? base64Chars.charAt(char3) : '=') +\n(i < str.length + 1 ? base64Chars.charAt(char4) : '='); \n } \n\n  return result; \n } \n\nfunction compute(payload) { \n  const credentials = payload.clientId + \":\" + payload.clientSecret;\n  const encodedCredentials = btoa(credentials);\n  return \"Basic \" + encodedCredentials;\n}; function headers(payload) { const credentials = payload.clientId + \":\" + payload.clientSecret; const encodedCredentials = btoa(credentials); return { authorization: \"Basic \" + encodedCredentials }; }; function body(payload) { const body = {grant_type: \"authorization_code\", code: payload.metadata.code, redirect_uri: payload.metadata.redirectUri}; return body; }; function compute(payload) { return { headers: headers(payload), body: body(payload) }; };",
        "responseCompute": "function compute(payload) { return { accessToken: payload.access_token, refreshToken: payload.refresh_token, expiresIn: payload.expires_in, tokenType: payload.token_type }; }"
    }
}
```

### `v1/oauth/:platform` Connection OAuth Definitions

### `POST` Requests

The `POST` request have the format:

```json
{
    "connectionDefinitionId": "conn_def::F6MxYHq3G2k::8ZIUdCAXTr-dX_CCBXeQDQ",
    "payload": {
        "code": "bteTDmQWsmEdKJAwt_AoCESx5GKWO-L6ysZ_6szzdIM",
        "redirectUri": "http://localhost:34676/callback"
    },
    "clientId": "D2E9E9AFED384A248654176E2D0FBA63",
    "group": "{{$randomAbbreviation}}",
    "label": "{{$randomAbbreviation}}"
}
```

### `/v1/connection-definitions` Connection Definitions

The `GET`, `DELETE`, and `POST` responses have the format:

```json
{
  "_id": "conn_def::F4-WTUpbMag::SmHUiaVsQLGsyW5VPcj_bw",
  "platformVersion": "BHm3CLrgmibci40LNu",
  "platform": "n5Y5PX92TnHQj08",
  "type": "custom",
  "name": "UnMqJY",
  "authSecrets": [],
  "authMethod": null,
  "frontend": {
    "spec": {
      "title": "UnMqJY",
      "description": "fiYnwI3DTcD7BoW7",
      "platform": "n5Y5PX92TnHQj08",
      "category": "yd6q6MPz",
      "image": "GLDuS",
      "tags": ["DbrJIVq", "quWjC6limknKTI9C5I", "3VMCK5Cv3N", "A7tt1X"]
    },
    "connectionForm": {
      "name": "Connect",
      "description": "Securely connect your account",
      "formData": []
    }
  },
  "paths": {
    "id": null,
    "event": null,
    "payload": null,
    "timestamp": "GoCqjy7WcwiDlgs",
    "secret": "0JxyiTAUDNW",
    "signature": null,
    "cursor": null
  },
  "settings": {
    "parseWebhookBody": true,
    "showSecret": false,
    "allowCustomEvents": false,
    "oauth": {
      "type": "disabled"
    }
  },
  "hidden": false,
  "testConnection": null,
  "createdAt": 1697740843246,
  "updatedAt": 1697740843246,
  "updated": false,
  "version": "1.0.0",
  "lastModifiedBy": "system",
  "deleted": false,
  "changeLog": {},
  "tags": [],
  "active": true,
  "deprecated": false
}
```

The `POST` and `PATCH` payloads have the format:

```json
{
  "platform": "n5Y5PX92TnHQj08",
  "platformVersion": "BHm3CLrgmibci40LNu",
  "type": "custom",
  "name": "UnMqJY",
  "description": "fiYnwI3DTcD7BoW7",
  "category": "yd6q6MPz",
  "image": "GLDuS",
  "tags": ["DbrJIVq", "quWjC6limknKTI9C5I", "3VMCK5Cv3N", "A7tt1X"],
  "authentication": [],
  "settings": {
    "parseWebhookBody": true,
    "showSecret": false,
    "allowCustomEvents": false,
    "oauth": {
      "type": "disabled"
    }
  },
  "paths": {
    "id": null,
    "event": null,
    "payload": null,
    "timestamp": "GoCqjy7WcwiDlgs",
    "secret": "0JxyiTAUDNW",
    "signature": null,
    "cursor": null
  },
  "testConnection": "conn_mod_def::F5zMkdbRJdc::vhALlwvZR6aI2U8Ub7xCHg",
  "active": true
}
```

### `/v1/connection-model-definitions` Connection Model Definitions

The `GET`, `DELETE`, and `POST` responses have the format:

```json
{
  "_id": "conn_mod_def::F4-WYWQcQjA::DsGW0UWlQe2sUQNe6Pp12g",
  "connectionPlatform": "ZJmUuHVr2AN",
  "connectionDefinitionId": "tx::Tfow-4FtI9c::QrH-T0eCwk8ql2jSXZr4PA",
  "platformVersion": "k3YXF8YWytA4qFeN",
  "title": "n0eIi",
  "name": "OohEHl1eU6a69",
  "action": "GET",
  "baseUrl": "kEPVIiax",
  "path": "LQ37My6",
  "authMethod": {
    "type": "BasicAuth",
    "username": "pr7kp7Yw2L",
    "password": "bNsRtgvoZdpp3s1"
  },
  "headers": null,
  "queryParams": null,
  "samples": {},
  "schemas": {
    "type": "object",
    "properties": {}
  },
  "paths": {
    "request": {
      "object": "$.body.order"
    },
    "response": {
      "object": "$.body.order",
      "id": "$.body.order.id",
      "cursor": null
    }
  },
  "testConnectionStatus": {
    "lastTestedAt": 0,
    "state": "untested"
  },
  "createdAt": 1697740929577,
  "updatedAt": 1697740929577,
  "updated": false,
  "version": "6.8.14",
  "lastModifiedBy": "system",
  "deleted": false,
  "changeLog": {},
  "tags": [],
  "active": true,
  "deprecated": false
}
```

The `POST` and `PATCH` payloads have the format:

```json
{
  "connectionPlatform": "ZJmUuHVr2AN",
  "connectionDefinitionId": "tx::Tfow-4FtI9c::QrH-T0eCwk8ql2jSXZr4PA",
  "platformVersion": "k3YXF8YWytA4qFeN",
  "title": "n0eIi",
  "name": "OohEHl1eU6a69",
  "baseUrl": "kEPVIiax",
  "path": "LQ37My6",
  "authMethod": {
    "type": "BasicAuth",
    "username": "pr7kp7Yw2L",
    "password": "bNsRtgvoZdpp3s1"
  },
  "paths": {
    "request": {
      "object": "$.body.order"
    },
    "response": {
      "object": "$.body.order",
      "id": "$.body.order.id",
      "cursor": null
    }
  },
  "action": "GET",
  "headers": null,
  "queryParams": null,
  "samples": {},
  "schemas": {
    "type": "object",
    "properties": {}
  },
  "version": "6.8.14"
}
```

### `/v1/connection-model-definitions/test/:id` Connection Model Definitions

The `POST` request has the format:

```json
{
  "connectionKey": "shopify::testing-connection",
  "request": {
    "headers": {
      "Content-Type": "application/json"
    },
    "queryParams": {},
    "pathParams": {
      "api_version": "2023-10"
    },
    "body": {
      "customer": {
        "first_name": "Steve2",
        "last_name": "Lastnameson2",
        "email": "steve.lastnameson3@example.com",
        "phone": "+15142543211",
        "verified_email": true,
        "addresses": [
          {
            "address1": "123 Oak St",
            "city": "Ottawa",
            "province": "ON",
            "phone": "555-1212",
            "zip": "123 ABC",
            "last_name": "Lastnameson",
            "first_name": "Mother",
            "country": "CA"
          }
        ],
        "password": "newpass",
        "password_confirmation": "newpass",
        "send_email_welcome": false
      }
    }
  }
}
```

Success Response:

```json
{
  "code": 201,
  "status": {
    "lastTestedAt": 1701269093942,
    "state": "success"
  },
  "meta": {
    "timestamp": 1701269093944,
    "platform": "shopify",
    "platformVersion": "2023-10",
    "connectionDefinitionId": "conn_def::F5mzNk_Tt9A::aXpKo-F3SAaiQVD16Q__nA",
    "connectionKey": "shopify::testing-connection",
    "modelName": "create_customer",
    "action": "POST"
  },
  "response": "{\"customer\":{...}}"
}
```

Failure Response:

```json
{
  "code": 400,
  "status": {
    "lastTestedAt": 1701269121866,
    "state": {
      "failure": {
        "message": "Bad Request"
      }
    }
  },
  "meta": {
    "timestamp": 1701269121869,
    "platform": "shopify",
    "platformVersion": "2023-10",
    "connectionDefinitionId": "conn_def::F5mzNk_Tt9A::aXpKo-F3SAaiQVD16Q__nA",
    "connectionKey": "shopify::testing-connection",
    "modelName": "customers",
    "action": "GET"
  },
  "response": "Bad Request"
}
```

### `/v1/connection-model-schemas` Connection Model Schemas

The `GET`, `DELETE`, and `POST` responses have the format:

```json
{
  "_id": "conn_mod_sch::F4-WaNQisZg::AEAMd-pwR7KgCOXErebXaQ",
  "platformId": "job::KhzJ3k6uT5g::YDe89fFipCILugI93iUvEQ",
  "platformPageId": "pipe::hyTA__88knM::LfKo5A30Q26Sd793-W1Tvg",
  "connectionPlatform": "AQI8AlWVJpH3KWCbJ",
  "connectionDefinitionId": "conn_def::9vWLYStbVPk::CfjmdRFAixElTxEQ0GLtVA",
  "platformVersion": "Q4e9PAKrcat",
  "modelName": "M6he0O",
  "schema": {
    "type": "YOJLofxSfaK",
    "properties": {
      "U79rxjh9yu0Pwt": {
        "type": "EgMEKvSd",
        "path": "oUWgRmSxZXswD",
        "description": "QDPEN4sC"
      },
      "DnnXOSX5Mbg": {
        "type": "99jq4t11EleVV",
        "path": "vJQVV6woZ",
        "description": null
      },
      "FI5lEp": {
        "type": "kQn8HgiSEM5",
        "path": null,
        "description": null
      },
      "7wN0c": {
        "type": "uW5y5z9",
        "path": "Gj8aqaVpMnXKR",
        "description": null
      },
      "i5KHq7jdoTyDwsAiGs": {
        "type": "hjcgwrNf",
        "path": "9yZhifJsNb",
        "description": null
      },
      "0bbcW": {
        "type": "sPDF1HnnuNASVy",
        "path": null,
        "description": null
      }
    },
    "required": null
  },
  "paths": {
    "id": "$.id",
    "createdAt": "$.created_at",
    "updatedAt": null
  },
  "mapping": null,
  "createdAt": 1697740961521,
  "updatedAt": 1697740961521,
  "updated": false,
  "version": "1.0.0",
  "lastModifiedBy": "system",
  "deleted": false,
  "changeLog": {},
  "tags": [],
  "active": true,
  "deprecated": false
}
```

The `POST` and `PATCH` payloads have the format:

```json
{
  "platformId": "job::KhzJ3k6uT5g::YDe89fFipCILugI93iUvEQ",
  "platformPageId": "pipe::hyTA__88knM::LfKo5A30Q26Sd793-W1Tvg",
  "connectionPlatform": "AQI8AlWVJpH3KWCbJ",
  "connectionDefinitionId": "conn_def::9vWLYStbVPk::CfjmdRFAixElTxEQ0GLtVA",
  "platformVersion": "Q4e9PAKrcat",
  "modelName": "M6he0O",
  "schema": {
    "type": "YOJLofxSfaK",
    "properties": {
      "DnnXOSX5Mbg": {
        "type": "99jq4t11EleVV",
        "path": "vJQVV6woZ",
        "description": null
      },
      "U79rxjh9yu0Pwt": {
        "type": "EgMEKvSd",
        "path": "oUWgRmSxZXswD",
        "description": "QDPEN4sC"
      },
      "FI5lEp": {
        "type": "kQn8HgiSEM5",
        "path": null,
        "description": null
      },
      "7wN0c": {
        "type": "uW5y5z9",
        "path": "Gj8aqaVpMnXKR",
        "description": null
      },
      "0bbcW": {
        "type": "sPDF1HnnuNASVy",
        "path": null,
        "description": null
      },
      "i5KHq7jdoTyDwsAiGs": {
        "type": "hjcgwrNf",
        "path": "9yZhifJsNb",
        "description": null
      }
    },
    "required": null
  },
  "paths": {
    "id": "$.id",
    "createdAt": "$.created_at",
    "updatedAt": null
  },
  "mapping": null
}
```

`GET`` / (Common CRUD Endpoints):

```json
{
  "connectionDefinitionId": "conn_def::F5wy_4FXeoA::r1ZvYSOASBivcMw1Triu1Q"
}
```

```json
[
  {
    "_id": "conn_mod_sch::F5wzdAvvBM8::_xSJQALySR-bQ_aFFnfyow",
    "connectionPlatform": "shopify",
    "connectionDefinitionId": "conn_def::F5wy_4FXeoA::r1ZvYSODSBivcMw1Triu1Q",
    "platformVersion": "2023-10",
    "modelName": "DiscountCode",
    "mapping": {
      "commonModelName": "GiftCards"
    },
    "createdAt": 1701291332748,
    "updatedAt": 1701291332748,
    "updated": false,
    "version": "1.0.0",
    "lastModifiedBy": "system",
    "deleted": false,
    "changeLog": {},
    "tags": [],
    "active": true,
    "deprecated": false
  }
]
```

### `/v1/common-models` Common Models

The `GET`, `DELETE`, and `POST` responses have the format:

```json
{
  "_id": "gm::F4-WcODfqMA::fCdamKVUTpi4k5uZRwpJbg",
  "name": "caohE4",
  "fields": [
    {
      "name": "0LAwsHhUY",
      "datatype": "Date"
    },
    {
      "name": "CDyzz6pBp",
      "datatype": "Date"
    },
    {
      "name": "Iz7ofBo9oxAGOsv",
      "datatype": "String"
    },
    {
      "name": "tYfojTeHq2Oby",
      "datatype": "Enum",
      "options": ["3qCLm1Ifg"]
    },
    {
      "name": "ij2C6Ytfhqhnmdz522",
      "datatype": "Array",
      "elementType": {
        "datatype": "Array",
        "elementType": {
          "datatype": "Date"
        }
      }
    },
    {
      "name": "Ywe0UZlRIhQqfePZZo",
      "datatype": "Boolean"
    },
    {
      "name": "IXLErlwISvzKxRw5",
      "datatype": "Expandable",
      "reference": "YosqAgwYC"
    },
    {
      "name": "5xH2RzL3WAwWGJ",
      "datatype": "Number"
    }
  ],
  "category": "gcTGn0lM7F7R8xe",
  "createdAt": 1697740996095,
  "updatedAt": 1697740996095,
  "updated": false,
  "version": "7.6.13",
  "lastModifiedBy": "system",
  "deleted": false,
  "changeLog": {},
  "tags": [],
  "active": true,
  "deprecated": false
}
```

The `POST` and `PATCH` payloads have the format:

```json
{
  "name": "caohE4",
  "version": "7.6.13",
  "fields": [
    {
      "name": "0LAwsHhUY",
      "datatype": "Date"
    },
    {
      "name": "CDyzz6pBp",
      "datatype": "Date"
    },
    {
      "name": "Iz7ofBo9oxAGOsv",
      "datatype": "String"
    },
    {
      "name": "tYfojTeHq2Oby",
      "datatype": "Enum",
      "options": ["3qCLm1Ifg"]
    },
    {
      "name": "ij2C6Ytfhqhnmdz522",
      "datatype": "Array",
      "elementType": {
        "datatype": "Array",
        "elementType": {
          "datatype": "Date"
        }
      }
    },
    {
      "name": "Ywe0UZlRIhQqfePZZo",
      "datatype": "Boolean"
    },
    {
      "name": "IXLErlwISvzKxRw5",
      "datatype": "Expandable",
      "reference": "YosqAgwYC"
    },
    {
      "name": "5xH2RzL3WAwWGJ",
      "datatype": "Number"
    }
  ],
  "category": "gcTGn0lM7F7R8xe"
}
```

### `GET /v1/common-models/:id/expand`

Returns the common model referenced by `:id` with all `expandable` fields expanded recursively.

The response will have a format similar to the following:

```json
{
  "_id": "cm::F5II13XsG6g::hr2Usj-TSbS7MISxCzrD_A",
  "name": "8WuZHvoVXO",
  "fields": [
    {
      "name": "870hOUP19sN3Ldp",
      "datatype": "Expandable",
      "reference": "S9UN7lXUwfdrttJe",
      "model": {
        "_id": "cm::F5II13VDwgA::QFjFJN4gS0y0STwmYYVXjQ",
        "name": "S9UN7lXUwfdrttJe",
        "fields": [],
        "category": "9c26GTSTglf8n",
        "createdAt": 1698429730950,
        "updatedAt": 1698429730950,
        "updated": false,
        "version": "7.5.7",
        "lastModifiedBy": "system",
        "deleted": false,
        "changeLog": {},
        "tags": [],
        "active": true,
        "deprecated": false
      }
    },
    {
      "name": "Ze3qMkMRC",
      "datatype": "Array",
      "elementType": {
        "datatype": "Expandable",
        "reference": "S9UN7lXUwfdrttJe",
        "model": {
          "_id": "cm::F5II13VDwgA::QFjFJN4gS0y0STwmYYVXjQ",
          "name": "S9UN7lXUwfdrttJe",
          "fields": [],
          "category": "9c26GTSTglf8n",
          "createdAt": 1698429730950,
          "updatedAt": 1698429730950,
          "updated": false,
          "version": "7.5.7",
          "lastModifiedBy": "system",
          "deleted": false,
          "changeLog": {},
          "tags": [],
          "active": true,
          "deprecated": false
        }
      }
    }
  ],
  "category": "2UBilcFRq4ymekG",
  "createdAt": 1698429730961,
  "updatedAt": 1698429730961,
  "updated": false,
  "version": "8.8.10",
  "lastModifiedBy": "system",
  "deleted": false,
  "changeLog": {},
  "tags": [],
  "active": true,
  "deprecated": false
}
```
