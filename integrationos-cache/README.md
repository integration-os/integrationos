# Pica Cache

A minimal wrapper around Moka and Redis for caching in the Pica project.

## Purpose

Pica Cache provides a lightweight, minimalistic interface to manage caching operations using Moka and Redis. While it is not runnable as a standalone service, it can be integrated into other systems and thoroughly tested to ensure reliable caching behavior.

## Running the Tests

To run the test suite for the cache system, use the following command:

```bash
cargo nextest run --all-features
```

This command runs the entire test suite, validating the integration between Moka, Redis, and the cache wrapper.
