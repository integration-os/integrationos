# Event Core

Processes incoming events from the [gateway](../gateway/) over redis and executes the associated pipelines.

## Dependencies

Requires redis to receive events from the [gateway](../gateway).

```bash
$ docker run -p 6379:6379 redis
```

Requires mongodb.

```bash
$ docker run -p 27017:27017 mongodb
```

Connecting to an external mongodb requires setting multiple environment variables in your `.env` file depending on which db you want to use.

`"CONTROL_DATABASE_URL"` and `"CONTROL_DATABASE_NAME"` are for the db which stores integration records.
`"EVENT_DATABASE_URL"` and `"EVENT_DATABASE_NAME"` are for the db which stores events.
`"CONTEXT_DATABASE_URL"` and `"CONTEXT_DATABASE_NAME"` are for the db which will store contexts and event-transactions.

## Running

```bash
$ cargo run
```

By default this will log everything, including dependencies, at the `DEBUG` level. To do more granular filtering, you can set the `RUST_LOG` environment variable in the `.env` file or in the command line such as:

```bash
$ RUST_LOG=event_core=info cargo run
```

which will output logs from only this crate at the `INFO` level.
