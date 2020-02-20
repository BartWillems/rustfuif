embed_migrations!("migrations/");

use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use diesel::PgConnection;

pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

fn connect(database_url: &str) -> diesel::ConnectionResult<PgConnection> {
    PgConnection::establish(database_url)
}

pub fn migrate(database_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let connection = connect(database_url)?;
    embedded_migrations::run_with_output(&connection, &mut std::io::stdout())?;

    Ok(())
}

pub fn build_connection_pool(database_url: &str) -> Result<Pool, Box<dyn std::error::Error>> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool: Pool = r2d2::Pool::builder().build(manager)?;

    Ok(pool)
}
