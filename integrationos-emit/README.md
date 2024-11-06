# Integrationos Emit

## Architecture

![zenuml](https://github.com/user-attachments/assets/e8ac8908-77af-491c-8489-ebd20f17f06e)

## Running the Emitter

This guide assumes that you have already have a working MongoDB instance running.

1. Install [fluvio cli and setup Docker compose](https://www.fluvio.io/docs/fluvio/installation/docker/)
2. Create the topic you want to emit to by running the following command:

```bash
fluvio topic create <topic_name> -p <num_partitions>
```
3. Create the death letter queue topic you want to emit to by running the following command:

```bash 
fluvio topic create dlq -p <num_partitions>
```
4. Run the emitter with the following command:

```bash
$ cargo watch -x run -q | bunyan
```

This command will monitor changes in the project and execute the emitter service with Bunyan-formatted logging.

## Running the Tests

To run the tests for the emitter, use:

```bash
cargo nextest run --all-features
```

This will execute all tests in the project, ensuring that the archiving process works as expected across all features.

