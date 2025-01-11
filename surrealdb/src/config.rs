use crate::{Manager, Pool};
use deadpool_runtime::Runtime;
use std::time::Duration;

/// Authentication credentials for SurrealDB
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Credentials {
    /// Root user credentials
    Root {
        /// Username
        user: String,
        /// Password
        pass: String,
    },
    /// Namespace-scoped credentials
    Namespace {
        /// Username
        user: String,
        /// Password
        pass: String,
        /// Namespace
        ns: String,
    },
    /// Database-scoped credentials
    Database {
        /// Username
        user: String,
        /// Password
        pass: String,
        /// Namespace
        ns: String,
        /// Database
        db: String,
    },
}

/// Configuration for SurrealDB connection pool
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Config {
    /// Database host URL (e.g. "ws://localhost:8000" or "mem://")
    pub host: String,
    /// Default namespace
    pub ns: String,
    /// Default database
    pub db: String,
    /// Authentication credentials
    pub creds: Credentials,
    /// Connection timeout in seconds
    #[cfg_attr(feature = "serde", serde(default = "default_connect_timeout"))]
    pub connect_timeout: u64,
    /// Maximum number of connections in the pool
    #[cfg_attr(feature = "serde", serde(default = "default_max_connections"))]
    pub max_connections: u32,
    /// Idle timeout in seconds
    #[cfg_attr(feature = "serde", serde(skip))]
    pub idle_timeout: u64,
}

fn default_connect_timeout() -> u64 {
    5
}

fn default_idle_timeout() -> u64 {
    60
}

fn default_max_connections() -> u32 {
    10
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: String::new(),
            ns: "test".to_string(),
            db: "test".to_string(),
            creds: Credentials::Root {
                user: "root".to_string(),
                pass: "root".to_string(),
            },
            connect_timeout: default_connect_timeout(),
            max_connections: default_max_connections(),
            idle_timeout: default_idle_timeout(),
        }
    }
}

impl Config {
    /// Creates a new configuration with the specified parameters
    ///
    /// # Arguments
    ///
    /// * `host` - Database host URL (e.g. "ws://localhost:8000" or "mem://")
    /// * `ns` - Default namespace
    /// * `db` - Default database
    /// * `creds` - Authentication credentials
    ///
    /// # Returns
    ///
    /// A new Config instance with default timeouts and pool size
    pub fn new(host: String, ns: String, db: String, creds: Credentials) -> Self {
        Self {
            host,
            ns,
            db,
            creds,
            connect_timeout: default_connect_timeout(),
            max_connections: default_max_connections(),
            idle_timeout: default_idle_timeout(),
        }
    }

    /// Creates a new configuration builder
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::new()
    }

    /// Get the connection timeout as a Duration
    pub fn connect_timeout(&self) -> Duration {
        Duration::from_secs(self.connect_timeout)
    }

    /// Get the idle timeout as a Duration
    pub fn idle_timeout(&self) -> Duration {
        Duration::from_secs(self.idle_timeout)
    }

    /// Creates a new connection pool with the given runtime
    pub fn create_pool(&self, runtime: Option<Runtime>) -> crate::Result<Pool> {
        let mgr = Manager::from_config(self);
        let builder = Pool::builder(mgr)
            .max_size(self.max_connections as usize)
            .wait_timeout(Some(Duration::from_secs(self.connect_timeout)))
            .create_timeout(Some(Duration::from_secs(self.connect_timeout)))
            .recycle_timeout(Some(Duration::from_secs(self.idle_timeout)));
        match runtime {
            Some(rt) => Ok(builder.runtime(rt).build()?),
            None => Ok(builder.build()?),
        }
    }
}

/// Builder for SurrealDB configuration
#[derive(Debug, Default)]
pub struct ConfigBuilder {
    host: Option<String>,
    ns: Option<String>,
    db: Option<String>,
    creds: Option<Credentials>,
    connect_timeout: Option<u64>,
    max_connections: Option<u32>,
    idle_timeout: Option<u64>,
}

impl ConfigBuilder {
    /// Creates a new configuration builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the database host URL
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    /// Sets the default namespace
    pub fn namespace(mut self, ns: impl Into<String>) -> Self {
        self.ns = Some(ns.into());
        self
    }

    /// Sets the default database
    pub fn database(mut self, db: impl Into<String>) -> Self {
        self.db = Some(db.into());
        self
    }

    /// Sets the authentication credentials
    pub fn credentials(mut self, creds: Credentials) -> Self {
        self.creds = Some(creds);
        self
    }

    /// Sets the connection timeout in seconds
    pub fn connect_timeout(mut self, timeout: u64) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Sets the maximum number of connections in the pool
    pub fn max_connections(mut self, max: u32) -> Self {
        self.max_connections = Some(max);
        self
    }

    /// Sets the idle timeout in seconds
    pub fn idle_timeout(mut self, timeout: u64) -> Self {
        self.idle_timeout = Some(timeout);
        self
    }

    /// Builds the configuration
    pub fn build(self) -> Result<Config, &'static str> {
        Ok(Config {
            host: self.host.ok_or("host is required")?,
            ns: self.ns.ok_or("namespace is required")?,
            db: self.db.ok_or("database is required")?,
            creds: self.creds.ok_or("credentials are required")?,
            connect_timeout: self.connect_timeout.unwrap_or_else(default_connect_timeout),
            max_connections: self.max_connections.unwrap_or_else(default_max_connections),
            idle_timeout: self.idle_timeout.unwrap_or_else(default_idle_timeout),
        })
    }
}