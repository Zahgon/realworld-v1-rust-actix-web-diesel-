use crate::constants::env_key;
use dotenv::dotenv;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::env;

pub type DbPool = PgPool;

async fn init_pool(database_url: &str) -> Result<DbPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
}

pub async fn establish_connection() -> DbPool {
    dotenv().ok();
    let database_url = env::var(env_key::DATABASE_URL).expect("DATABASE_URL must be set");
    init_pool(&database_url).await.expect("Failed to create pool")
}
