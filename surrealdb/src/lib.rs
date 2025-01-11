//! A connection pool implementation for SurrealDB.
//!
//! # Example
//!
//! ```rust,no_run
//!
//! let config = deadpool_surrealdb::Config {
//!     host: "mem://".to_string(),
//!     ns: "test".to_string(),
//!     db: "test".to_string(),
//!     connect_timeout: 5,
//!     idle_timeout: 10,
//!     max_connections: 16,
//!     creds: deadpool_surrealdb::Credentials::Root {
//!         user: String::new(),
//!         pass: String::new(),
//!     },
//! };
//!
//! let pool = config.create_pool(Some(deadpool_surrealdb::Runtime::Tokio1)).unwrap();
//! ```
//!
//! # Features
//!
//! - Connection pooling
//! - Automatic connection recycling
//! - Support for different runtimes (Tokio, async-std)
//! - Support for different authentication methods
//! - Support for connection timeouts and idle timeouts
//! - Support for maximum connections limit
//!
//! # Runtime Support
//!
//! The following runtimes are supported:
//!
//! - `tokio1` - Tokio 1.x
//! - `async-std1` - async-std 1.x
//!
//! # Authentication Methods
//!
//! The following authentication methods are supported:
//!
//! - Root user authentication
//! - Namespace user authentication
//! - Database user authentication
//! - Scope user authentication
//!
//! # Configuration
//!
//! The pool can be configured using the `Config` struct:
//!
//! ```rust,no_run
//!
//! let config = deadpool_surrealdb::Config {
//!     host: "mem://".to_string(),
//!     ns: "test".to_string(),
//!     db: "test".to_string(),
//!     connect_timeout: 5,
//!     idle_timeout: 10,
//!     max_connections: 16,
//!     creds: deadpool_surrealdb::Credentials::Root {
//!         user: String::new(),
//!         pass: String::new(),
//!     },
//! };
//! ```

#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(
    nonstandard_style,
    rust_2018_idioms,
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links
)]
#![forbid(non_ascii_idents, unsafe_code)]
#![warn(
    deprecated_in_future,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    unused_import_braces,
    unused_labels,
    unused_lifetimes,
    unused_qualifications,
    unused_results
)]
#![allow(clippy::uninlined_format_args)]

/// Configuration types for the SurrealDB connection pool.
pub mod config;

use deadpool::managed;
use std::borrow::Cow;
use surrealdb::{
    engine::any::Any,
    opt::auth,
    Surreal,
};
use deadpool::managed::RecycleError;

deadpool::managed_reexports!(
    "surrealdb",
    Manager,
    managed::Object<Manager>,
    Error,
    std::convert::Infallible
);
pub use self::{config::Config, config::Credentials};
pub use deadpool_runtime::Runtime;

/// Error type for SurrealDB pool operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// SurrealDB error
    #[error("SurrealDB error: {0}")]
    Surreal(#[from] surrealdb::Error),
    
    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),
    
    /// Authentication error
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Build error
    #[error("Build error: {0}")]
    Build(#[from] managed::BuildError),
}

/// Result type for SurrealDB pool operations
pub type Result<T> = std::result::Result<T, Error>;

/// Manager for creating and recycling SurrealDB connections.
#[derive(Debug)]
pub struct Manager {
    config: Config,
}

impl Manager {
    /// Creates a new Manager using the given Config.
    #[must_use]
    pub fn from_config(config: &Config) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Authenticate the connection using configured credentials
    async fn auth(&self, db: &Surreal<Any>) -> Result<()> {
        match &self.config.creds {
            Credentials::Root { user, pass } => {
                let _jwt = db.signin(auth::Root {
                    username: user,
                    password: pass,
                })
                .await
                .map_err(|e| Error::Auth(format!("Root auth failed: {}", e)))?;
            }
            Credentials::Namespace { user, pass, ns } => {
                let _jwt = db.signin(auth::Namespace {
                    username: user,
                    password: pass,
                    namespace: ns,
                })
                .await
                .map_err(|e| Error::Auth(format!("Namespace auth failed: {}", e)))?;
            }
            Credentials::Database {
                user,
                pass,
                ns,
                db: database,
            } => {
                let _jwt = db.signin(auth::Database {
                    username: user,
                    password: pass,
                    namespace: ns,
                    database,
                })
                .await
                .map_err(|e| Error::Auth(format!("Database auth failed: {}", e)))?;
            }
        }
        
        // Set namespace and database
        db.use_ns(&self.config.ns)
            .use_db(&self.config.db)
            .await
            .map_err(|e| Error::Connection(format!("Failed to set ns/db: {}", e)))?;
            
        Ok(())
    }
}

impl managed::Manager for Manager {
    type Type = Surreal<Any>;
    type Error = Error;

    async fn create(&self) -> Result<Self::Type> {
        // Connect to database
        let db = surrealdb::engine::any::connect(&self.config.host)
            .await
            .map_err(|e| Error::Connection(format!("Failed to connect: {}", e)))?;
            
        // Skip authentication for memory database
        if !self.config.host.starts_with("mem://") {
            // Authenticate
            self.auth(&db).await?;
        }
        
        // Set namespace and database
        db.use_ns(&self.config.ns)
            .use_db(&self.config.db)
            .await
            .map_err(|e| Error::Connection(format!("Failed to set ns/db: {}", e)))?;
            
        Ok(db)
    }

    async fn recycle(
        &self,
        conn: &mut Self::Type,
        _: &managed::Metrics,
    ) -> managed::RecycleResult<Self::Error> {
        // Skip authentication for memory database
        if !self.config.host.starts_with("mem://") {
            // Check connection health
            self.auth(conn)
                .await
                .map_err(|e| RecycleError::Message(Cow::Owned(format!("Connection check failed: {}", e))))?;
        }
            
        Ok(())
    }
}
