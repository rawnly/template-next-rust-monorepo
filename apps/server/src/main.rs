use clap::Parser;
use dotenv::dotenv;
use server::{config::Config, http};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    env_logger::init();

    let config = Config::parse();

    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;

    // this embeds database migration in the application binary
    // so we can ensure the db is migrated correctly on startup
    sqlx::migrate!().run(&db).await?;

    // spin up API
    http::serve(config, db).await?;

    Ok(())
}
