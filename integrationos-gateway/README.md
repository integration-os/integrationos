# IntegrationOS Gateway

Receives events by POSTing to the `/emit/:access_key` endpoint. Validates the access key and then stores the event in mongodb and transmits it over redis.

## Dependencies

Requires redis to send events to the [integrationos-event](../integrationos-event) service.

```bash
$ docker run -p 6379:6379 redis
```

Requires mongodb.

```bash
$ docker run -p 27017:27017 mongo
```

Connecting to an external mongodb requires setting some environment variables in your `.env` file depending on which db you want to use.

`"EVENT_DATABASE_URL"` and `"EVENT_DATABASE_NAME"` are for the db which stores events.

## Running

```bash
$ cargo run
```

By default this will log everything, including dependencies, at the `DEBUG` level. To do more granular filtering, you can set the `RUST_LOG` environment variable in the `.env` file or in the command line such as:

```bash
$ RUST_LOG=gateway=info cargo run
```

which will output logs from only this crate at the `INFO` level.
