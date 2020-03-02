embed_migrations!("migrations/");

use diesel::r2d2::{self, ConnectionManager};
use diesel::PgConnection;

/// Pooled postgres connections
pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

/// Connection received from the pool
pub type Conn = r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::PgConnection>>;

/// Attempts to run the pending embedded migrations
pub fn migrate(pool: &Pool) -> Result<(), Box<dyn std::error::Error>> {
    let conn = pool.get()?;

    embedded_migrations::run_with_output(&conn, &mut std::io::stdout())?;

    Ok(())
}

/// Builds a connection pool
pub fn build_connection_pool(database_url: &str) -> Result<Pool, Box<dyn std::error::Error>> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    debug!("attempting to connect to the database...");
    let pool: Pool = r2d2::Pool::builder().build(manager)?;
    debug!("connected!");

    Ok(pool)
}
