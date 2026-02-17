use rusqlite::Connection;
use std::sync::Mutex;

const DB_PATH: &str = "bluetooth_scan.db";

/// Database connection pool - ensures single connection is used across all operations.
///
/// This prevents:
/// - Multiple file locks on SQLite
/// - Race conditions between writes
/// - Performance degradation from repeated open/close
///
/// # Example
/// ```rust
/// // Initialize pool at startup
/// db_pool::init_pool()?;
///
/// // Use pooled connection
/// if let Some(pool) = db_pool::get_pool() {
///     pool.execute(|conn| {
///         // use conn here
///     })?;
/// }
/// ```
pub struct DbPool {
    conn: Mutex<Connection>,
}

impl DbPool {
    /// Creates a new database pool with a single shared connection.
    ///
    /// Opens the database and enables WAL mode for better concurrency.
    ///
    /// # Returns
    /// Result<Self, rusqlite::Error> - Pool instance or error
    pub fn new() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(DB_PATH)?;

        // Enable WAL mode for better concurrency
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Gets the connection for reading.
    ///
    /// Returns a guard that holds the database connection lock.
    /// The lock is released when the guard is dropped.
    ///
    /// # Returns
    /// DbGuard - Guard containing the connection
    pub fn get(&self) -> DbGuard<'_> {
        DbGuard {
            conn: self.conn.lock().unwrap(),
        }
    }

    /// Executes a function with connection access.
    ///
    /// Acquires the lock, runs the function, and releases the lock.
    ///
    /// # Arguments
    /// * `f` - Function that takes a &Connection and returns Result<T, Error>
    ///
    /// # Returns
    /// Result<T, rusqlite::Error> - Result from the function
    pub fn execute<F, T>(&self, f: F) -> Result<T, rusqlite::Error>
    where
        F: FnOnce(&Connection) -> Result<T, rusqlite::Error>,
    {
        let conn = self.conn.lock().unwrap();
        f(&conn)
    }
}

/// Guard that holds the database connection lock.
///
/// Provides safe access to the connection with automatic unlock on drop.
///
/// # Example
/// ```rust
/// let guard = pool.get();
/// let conn = guard.as_conn();
/// // use conn
/// // lock automatically released when guard drops
/// ```
pub struct DbGuard<'a> {
    conn: std::sync::MutexGuard<'a, Connection>,
}

impl<'a> DbGuard<'a> {
    /// Gets immutable reference to connection.
    ///
    /// # Returns
    /// &Connection - Immutable reference to the database connection
    pub fn as_conn(&self) -> &Connection {
        &self.conn
    }

    /// Gets mutable reference to connection.
    ///
    /// # Returns
    /// &mut Connection - Mutable reference to the database connection
    pub fn as_conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }
}

/// Global database pool - initialized once at startup.
///
/// Uses OnceCell for thread-safe lazy initialization.
use once_cell::sync::OnceCell;
use std::sync::Arc;

static DB_POOL: OnceCell<Arc<DbPool>> = OnceCell::new();

/// Initializes the global database pool.
///
/// Call this once at application startup before using any pooled functions.
///
/// # Returns
/// Result<(), rusqlite::Error> - Ok on success
pub fn init_pool() -> Result<(), rusqlite::Error> {
    let pool = DbPool::new()?;
    DB_POOL.set(Arc::new(pool)).ok();
    Ok(())
}

/// Gets the global database pool.
///
/// Returns None if pool not yet initialized.
///
/// # Returns
/// Option<Arc<DbPool>> - Pool if initialized, None otherwise
pub fn get_pool() -> Option<Arc<DbPool>> {
    DB_POOL.get().cloned()
}

/// Gets database pool or panics.
///
/// Use when pool must be initialized - panics if init_pool() not called.
///
/// # Returns
/// Arc<DbPool> - The database pool
///
/// # Panics
/// Panics if pool not initialized
pub fn pool() -> Arc<DbPool> {
    DB_POOL
        .get()
        .expect("Database pool not initialized - call init_pool() first")
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_creation() {
        let pool = DbPool::new();
        assert!(pool.is_ok());
    }
}
