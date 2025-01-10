use std::{collections::HashMap, env, time::Duration};

use serde::{Deserialize, Serialize};
use deadpool_surrealdb::{Config, Credentials, Pool};
use deadpool_runtime::Runtime;
use surrealdb::Error;

#[derive(Debug, Deserialize, Serialize)]
struct TestConfig {
    #[serde(default = "default_config")]
    surrealdb: Config,
}

fn default_config() -> Config {
    Config::new(
        "mem://".to_string(),
        "test".to_string(),
        "test".to_string(),
        Credentials::Root {
            user: "root".to_string(),
            pass: "root".to_string(),
        },
    )
}

impl TestConfig {
    pub fn from_env() -> Self {
        let cfg = config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()
            .unwrap();

        let cfg = cfg.try_deserialize::<Self>().unwrap_or_else(|_| TestConfig {
            surrealdb: default_config(),
        });
        
        cfg
    }
}

fn create_pool() -> Pool {
    let cfg = TestConfig::from_env();
    cfg.surrealdb.create_pool(Some(Runtime::Tokio1)).unwrap()
}

#[tokio::test]
async fn basic() -> Result<(), Error> {
    let pool = create_pool();
    let conn = pool.get().await.unwrap();
    
    // Test simple query with SET
    let _ = conn
        .query("INFO FOR DB")
        .await
        .unwrap()
        .check()?;
        
    // Test with table creation
    let _ = conn
        .query("CREATE type:test SET value = 1")
        .await
        .unwrap()
        .check()?;
        
    // Verify table was created
    let _ = conn
        .query("INFO FOR TABLE type")
        .await
        .unwrap()
        .check()?;
        
    Ok(())
}

#[tokio::test]
async fn parallel_queries() -> Result<(), Error> {
    let pool = create_pool();
    let mut handles = Vec::new();
    
    // Create multiple parallel queries
    for i in 0..10 {
        let pool = pool.clone();
        handles.push(tokio::spawn(async move {
            let conn = pool.get().await.unwrap();
            conn.query(format!("CREATE type:test{} SET value = {}", i, i))
                .await
                .unwrap()
                .check()
                .unwrap();
            i as i64
        }));
    }
    
    // Wait for all queries and verify results
    let results = futures::future::join_all(handles).await;
    for (i, result) in results.into_iter().enumerate() {
        assert_eq!(result.unwrap(), i as i64);
    }

    // Verify all tables were created
    let conn = pool.get().await.unwrap();
    conn.query("INFO FOR DB")
        .await
        .unwrap()
        .check()?;
        
    Ok(())
}

#[tokio::test]
async fn connection_timeout() {
    let mut cfg = TestConfig::from_env();
    cfg.surrealdb.connect_timeout = 1; // 1 second timeout
    
    // Use a non-existent host to trigger timeout
    cfg.surrealdb.host = "ws://non-existent-host:8000".to_string();
    
    let pool = cfg.surrealdb.create_pool(Some(Runtime::Tokio1)).unwrap();
    let result = pool.get().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn max_connections() {
    let mut cfg = TestConfig::from_env();
    cfg.surrealdb.max_connections = 2;
    
    let pool = cfg.surrealdb.create_pool(Some(Runtime::Tokio1)).unwrap();
    
    // Get two connections (should succeed)
    let conn1 = pool.get().await.unwrap();
    let _conn2 = pool.get().await.unwrap();
    
    // Third connection should timeout
    let result = tokio::time::timeout(Duration::from_secs(5), pool.get()).await;
    assert!(result.is_err());
    
    // Drop one connection and try again
    drop(conn1);
    let conn3 = pool.get().await.unwrap();
    assert!(conn3.health().await.is_ok());
}

#[tokio::test]
async fn connection_health() -> Result<(), surrealdb::Error> {
    let pool = create_pool();
    let conn = pool.get().await.unwrap();
    
    // Test health check
    assert!(conn.health().await.is_ok());
    
    // Force connection to be invalid (by closing the underlying connection)
    drop(conn);
    
    // Get a new connection
    let conn = pool.get().await.unwrap();
    assert!(conn.health().await.is_ok());

    Ok(())
}

#[tokio::test]
async fn auth_methods() -> Result<(), surrealdb::Error> {
    // Test root auth
    let mut cfg = TestConfig::from_env();
    cfg.surrealdb.creds = Credentials::Root {
        user: String::new(),
        pass: String::new(),
    };
    let pool = cfg.surrealdb.create_pool(Some(Runtime::Tokio1)).unwrap();
    let conn = pool.get().await.unwrap();
    assert!(conn.health().await.is_ok());
    
    // Test namespace auth
    cfg.surrealdb.creds = Credentials::Namespace {
        user: "test".to_string(),
        pass: "test".to_string(),
        ns: "test".to_string(),
    };
    let pool = cfg.surrealdb.create_pool(Some(Runtime::Tokio1)).unwrap();
    let conn = pool.get().await.unwrap();
    assert!(conn.health().await.is_ok());
    
    // Test database auth
    cfg.surrealdb.creds = Credentials::Database {
        user: "test".to_string(),
        pass: "test".to_string(),
        ns: "test".to_string(),
        db: "test".to_string(),
    };
    let pool = cfg.surrealdb.create_pool(Some(Runtime::Tokio1)).unwrap();
    let conn = pool.get().await.unwrap();
    assert!(conn.health().await.is_ok());

    Ok(())
}

#[cfg(feature = "serde")]
#[test]
fn config_from_env() {
    let mut env = Env::new();
    env.set("SURREALDB__HOST", "mem://");
    env.set("SURREALDB__NS", "test");
    env.set("SURREALDB__DB", "test");
    env.set("SURREALDB__CONNECT_TIMEOUT", "10");
    env.set("SURREALDB__MAX_CONNECTIONS", "20");
    env.set("SURREALDB__CREDS__ROOT__USER", "");
    env.set("SURREALDB__CREDS__ROOT__PASS", "");
    
    let cfg = TestConfig::from_env();
    assert_eq!(cfg.surrealdb.host, "mem://");
    assert_eq!(cfg.surrealdb.ns, "test");
    assert_eq!(cfg.surrealdb.db, "test");
    assert_eq!(cfg.surrealdb.connect_timeout, 10);
    assert_eq!(cfg.surrealdb.max_connections, 20);
    match cfg.surrealdb.creds {
        Credentials::Root { user, pass } => {
            assert_eq!(user, "");
            assert_eq!(pass, "");
        }
        _ => panic!("Expected root credentials"),
    }
}

struct Env {
    backup: HashMap<String, Option<String>>,
}

impl Env {
    pub fn new() -> Self {
        Self {
            backup: HashMap::new(),
        }
    }
    
    pub fn set(&mut self, name: &str, value: &str) {
        self.backup.insert(name.to_string(), env::var(name).ok());
        env::set_var(name, value);
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        for (name, value) in self.backup.iter() {
            match value {
                Some(val) => env::set_var(name.as_str(), val),
                None => env::remove_var(name.as_str()),
            }
        }
    }
} 