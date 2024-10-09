# IntegrationOS Storage

A versatile wrapper around multiple storage solutions, designed for single-tenant management of clients in the IntegrationOS project.

## Purpose

IntegrationOS Storage provides a unified interface for managing different storage backends, enabling single-tenant configurations for clients. It supports seamless integration of new databases, making it adaptable to various storage needs.

## Running the Storage Service

To run the storage service, use the following command:

```bash
$ cargo watch -x run -q | bunyan
```

By default, the service runs on port **5005**, but this can be configured through environment variables.

## Integrating a New Database

To add support for a new database, follow these steps:

1. **Add the database type to the supported list:**

   ```rust
   #[derive(Debug, Clone, PartialEq, Eq, EnumString, AsRefStr)]
   #[strum(serialize_all = "kebab-case")]
   pub enum StorageConfigType {
       Postgres,
       // Add new storage types here
   }
   ```

2. **Create the necessary configuration and add it to the configuration loader:**

   ```rust
   #[derive(Envconfig, Clone)]
   pub struct StorageConfig {
       #[envconfig(from = "WORKER_THREADS")]
       pub worker_threads: Option<usize>,
       #[envconfig(from = "INTERNAL_SERVER_ADDRESS", default = "0.0.0.0:5005")]
       pub address: SocketAddr,
       #[envconfig(from = "CACHE_SIZE", default = "100")]
       pub cache_size: u64,
       #[envconfig(nested = true)]
       pub cache: CacheConfig,
       #[envconfig(from = "ENVIRONMENT", default = "development")]
       pub environment: Environment,
       #[envconfig(nested = true)]
       pub postgres_config: PostgresConfig,
       #[envconfig(from = "STORAGE_CONFIG_TYPE", default = "postgres")]
       pub storage_config_type: StorageConfigType,
   }
   ```

3. **Implement the `Storage` trait:**

   ```rust
   #[async_trait]
   pub trait Storage: Send + Sync {
       async fn execute_raw(
           &self,
           query: &str,
       ) -> Result<Vec<HashMap<String, Value>>, IntegrationOSError>;
   }
   ```

   Be mindful that implementing this trait usually requires creating serializers for your specific storage type.

4. **Implement the `Initializer` trait:**

   ```rust
   #[async_trait]
   impl Initializer for PostgresStorage {
       async fn init(configuration: &StorageConfig) -> Result<Server, anyhow::Error> {
           let postgres: PostgresStorage = PostgresStorage::new(configuration).await?;
           let storage: Arc<dyn Storage> = Arc::new(postgres);

           Ok(Server {
               state: Arc::new(AppState {
                   config: configuration.clone(),
                   storage,
               }),
           })
       }
   }
   ```

After completing these steps, the compiler will guide you through the necessary changes to ensure the code compiles correctly. Remember to add the new
tests to verify the functionality of the new storage type.

## Running the Tests

To run the test suite for the storage service, execute:

```bash
cargo nextest run --all-features
```

This command will run all tests associated with the storage functionality, ensuring correct behavior across various scenarios.

