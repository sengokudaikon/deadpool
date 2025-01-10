# deadpool-surrealdb

Dead simple async pool for [SurrealDB](https://surrealdb.com).

## Features

- Async connection pooling for SurrealDB
- Support for multiple runtimes (tokio, async-std)
- Connection health checks and automatic recycling
- Configurable pool size and timeouts
- Support for different authentication methods (Root, Namespace, Database)

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
deadpool-surrealdb = "0.1"
```

### Example

```rust
use deadpool_surrealdb::{Manager, Pool, Config, Runtime, Credentials};

#[tokio::main]
async fn main() {
    // Create configuration
    let config = deadpool_surrealdb::Config {
        host: "mem://".to_string(),
        ns: "test".to_string(),
        db: "test".to_string(),
        connect_timeout: 5,
        idle_timeout: 10,
        max_connections: 16,
        creds: deadpool_surrealdb::Credentials::Root {
            user: String::new(),
            pass: String::new(),
        },
    };

    // Create pool manager
    let mgr = deadpool_surrealdb::Manager::from_config(&config);
    let pool = Pool::builder(mgr)
        .max_size(16)
        .build()
        .unwrap();

    // Get connection from pool
    let conn = pool.get().await.unwrap();
    
    // Use the connection
    let result: Vec<serde_json::Value> = conn
        .query("SELECT * FROM person")
        .await
        .unwrap()
        .take(0)
        .unwrap();
}
```

## Configuration

The pool can be configured using the `Config` struct:

```rust, no_run
let config = deadpool_surrealdb::Config {
        host: "mem://".to_string(), // SurrealDB connection url (ws://, wss://, mem://, etc)
        ns: "test".to_string(), // Namespace to use
        db: "test".to_string(), // Database to use
        connect_timeout: 5, // Connection timeout in seconds
        idle_timeout: 10, // Connection idle timeout in seconds
        max_connections: 16, // Maximum number of connections in the pool
        creds: deadpool_surrealdb::Credentials::Root { // Authentication credentials
            user: String::new(),
            pass: String::new(),
        },
    };
```

Authentication methods:

- Root: `Credentials::Root { user, pass }`
- Namespace: `Credentials::Namespace { user, pass, ns }`
- Database: `Credentials::Database { user, pass, ns, db }`

## Features

- `rt_tokio_1` - Enable tokio 1.x support (default)
- `rt_async-std_1` - Enable async-std 1.x support
- `serde` - Enable serde support for config serialization

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions. 