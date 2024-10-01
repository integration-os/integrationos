# IntegrationOS Unified

The core logic for unification APIs and user request handling in the IntegrationOS project.

## Purpose

IntegrationOS Unified provides the core functionality for managing and processing unification APIs, ensuring proper handling of user requests across the IntegrationOS ecosystem. While this service is not directly runnable, its logic forms the backbone of the unification process within the system.

For detailed usage and API references, visit the [API documentation](https://docs.integrationos.com/reference/list-connections).

## Running the Tests

To ensure the correctness of the unification logic, run the following test suite:

```bash
cargo nextest run --all-features
```

This command will execute all tests related to the unification APIs and user request handling, ensuring that the system behaves as expected under various scenarios.
