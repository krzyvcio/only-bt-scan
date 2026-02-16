use rusqlite::Connection;
use std::sync::Mutex;

const DB_PATH: &str = "bluetooth_scan.db";

/// Database connection pool - ensures single connection is used across all operations
///
/// This prevents:
/// - Multiple file locks on SQLite
/// - Race conditions between writes
/// - Performance degradation from repeated open/close
pub struct DbPool {
    conn: Mutex<Connection>,
}

impl DbPool {
    /// Create new database pool with single shared connection
    pub fn new() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(DB_PATH)?;

        // Enable WAL mode for better concurrency
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Get the connection for reading
    pub fn get(&self) -> DbGuard<'_> {
        DbGuard {
            conn: self.conn.lock().unwrap(),
        }
    }

    /// Execute a function with connection access
    pub fn execute<F, T>(&self, f: F) -> Result<T, rusqlite::Error>
    where
        F: FnOnce(&Connection) -> Result<T, rusqlite::Error>,
    {
        let conn = self.conn.lock().unwrap();
        f(&conn)
    }
}

/// Guard that holds the database connection lock
pub struct DbGuard<'a> {
    conn: std::sync::MutexGuard<'a, Connection>,
}

impl<'a> DbGuard<'a> {
    /// Get immutable reference to connection
    pub fn as_conn(&self) -> &Connection {
        &self.conn
    }

    /// Get mutable reference to connection  
    pub fn as_conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }
}

/// Global database pool - initialized once at startup
use once_cell::sync::OnceCell;
use std::sync::Arc;

static DB_POOL: OnceCell<Arc<DbPool>> = OnceCell::new();

/// Initialize the global database pool
pub fn init_pool() -> Result<(), rusqlite::Error> {
    let pool = DbPool::new()?;
    DB_POOL.set(Arc::new(pool)).ok();
    Ok(())
}

/// Get the global database pool
pub fn get_pool() -> Option<Arc<DbPool>> {
    DB_POOL.get().cloned()
}

/// Get database pool or panic (for cases where pool must be initialized)
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
