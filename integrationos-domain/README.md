# Pica Domain

This documentation does not aim to be an in-depth explanation of the code, but rather a high-level overview of the project.
For a more detailed explanation, please refer to the code itself.

## Overview

Pica domain seeks to hold the common data structures used on the [pica](https://github.com/picahq/pica) repository. Along with these DS, it also
has some utilities to create `id` and manipulate `json` as well as general purpose services.

### Environment Variables

The following environment variables are introduced, not necessarily used, by this project:

- `REDIS_URL`: The URL to connect to the Redis server. Default is `redis://localhost:6379`.
- `REDIS_QUEUE_NAME`: The name of the queue to be used in the Redis server. Default is `events`.
- `REDIS_EVENT_THROUGHPUT_KEY`: The key to be used to store the event throughput in the Redis server. Default is `event_throughput`.
- `REDIS_API_THROUGHPUT_KEY`: The key to be used to store the API throughput in the Redis server. Default is `api_throughput`.

- `CONTROL_DATABASE_URL`: The URL to connect to the control database. Default is `mongodb://localhost:27017`.
- `CONTROL_DATABASE_NAME`: The name of the control database. Default is `database`.
- `UDM_DATABASE_URL`: The URL to connect to the UDM database. Default is `mongodb://localhost:27017`.
- `UDM_DATABASE_NAME`: The name of the UDM database. Default is `udm`.
- `EVENT_DATABASE_URL`: The URL to connect to the event database. Default is `mongodb://localhost:27017`.
- `EVENT_DATABASE_NAME`: The name of the event database. Default is `database`.
- `CONTEXT_DATABASE_URL`: The URL to connect to the context database. Default is `mongodb://localhost:27017`.
- `CONTEXT_DATABASE_NAME`: The name of the context database. Default is `database`.
- `CONTEXT_COLLECTION_NAME`: The name of the context collection

- `ENVIRONMENT`: The environment in which the application is running. Default is `development`.

- `OPENAI_API_KEY`: The API key to connect to the OpenAI server

- `SECRETS_SERVICE_BASE_URL`: The base URL to connect to the secrets service. Default is `https://secrets-service-development-b2nnzrt2eq-uk.a.run.app/`.
- `SECRETS_SERVICE_GET_PATH`: The path to get secrets in the secrets service. Default is `v1/secrets/get/`.
- `SECRETS_SERVICE_CREATE_PATH`: The path to create secrets in the secrets service. Default is `v1/secrets/create/`.

- `WATCHDOG_EVENT_TIMEOUT`: The event timeout to be used in the watchdog service. Default is `300`.
- `WATCHDOG_POLL_DURATION`: The poll duration to be used in the watchdog service. Default is `10`.

### Services

- Caller Client: A client to make requests to external APIs. It is used to make requests to external APIs and return the response. It is used by the `pica` repository to make requests to external APIs.
- Secrets Client: A client to interact with the secrets service. It is used to get and create secrets in the secrets service. It is used by the `pica` repository to get and create secrets.
- Watchdog Client: A client to start and stop the watchdog service. It is used to start the watchdog service. It is used by the `pica` repository to start and stop the watchdog service.

### Data Structures

Please refer to the code itself for a detailed explanation of the data structures.

### Utilities

- Hash Data: A utility to hash data. It is used to hash data and return the hash. It is used by the `pica` repository to hash data.
