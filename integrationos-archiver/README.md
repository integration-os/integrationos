# IntegrationOS Archiver

A Rust-based archiver for the IntegrationOS project that archives the specified database.

## Purpose

IntegrationOS Archiver is a service designed to archive data from a specified database efficiently. It is built using Rust to ensure high performance and reliability in handling large volumes of data.

## Running the Archiver

To run the archiver, use the following command:

```bash
$ cargo watch -x run -q | bunyan
```

This command will monitor changes in the project and execute the archiver service with Bunyan-formatted logging.

## Running the Tests

To run the tests for the archiver, use:

```bash
cargo nextest run --all-features
```

This will execute all tests in the project, ensuring that the archiving process works as expected across all features.

