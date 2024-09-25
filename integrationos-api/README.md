## IntegrationOS API

Common API for the IntegrationOS project.

For a full list of endpoints, please refer to the following files:

- [Public Routes](./src/router/public.rs)
- [Private Routes](./src/router/secured_jwt.rs)
- [User Protected Routes](./src/router/secured_key.rs)

## Running the API

```bash
$ cargo watch -x run -q | bunyan
```

## Running the API with a specific configuration

Create a .env file in the root of the project with the following environment:

```bash
RUST_LOG=info
ENVIRONMENT=development
EVENT_DATABASE_URL=mongodb://localhost:27017/?directConnection=true
CONTROL_DATABASE_URL=mongodb://localhost:27017/?directConnection=true
CONTEXT_DATABASE_URL=mongodb://localhost:27017/?directConnection=true
UDM_DATABASE_URL=mongodb://localhost:27017/?directConnection=true
EVENT_DATABASE_NAME=events-service
CONTEXT_DATABASE_NAME=events-service
CONTROL_DATABASE_NAME=events-service
UDM_DATABASE_NAME=events-service
```

Then run the following command:

```bash
$ cargo watch -x run -q | bunyan
```

## Running the API tests

```bash
cargo nextest run --all-features
```
