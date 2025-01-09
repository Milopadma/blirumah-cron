use anyhow::Result;
use dotenv::dotenv;
use job_scheduler::{Job, JobScheduler};
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, types::time::OffsetDateTime, Pool, Postgres};
use std::{env, sync::Arc, time::Duration};

const CURRENCY_API_URL: &str = "https://api.currencyapi.com/v3/latest";

async fn fetch_currency_rates() -> Result<Value> {
    let api_key = env::var("CURRENCY_API_KEY")?;
    let client = reqwest::Client::new();
    let response = client
        .get(CURRENCY_API_URL)
        .query(&[("base_currency", "IDR")])
        .header("apikey", api_key)
        .send()
        .await?
        .json::<Value>()
        .await?;

    Ok(response["data"].clone())
}

async fn update_currency_rates(pool: &Pool<Postgres>) -> Result<()> {
    let rates = fetch_currency_rates().await?;
    let now = OffsetDateTime::now_utc();

    sqlx::query(
        r#"
        INSERT INTO currency_rates (rates, last_updated)
        VALUES ($1, $2)
        "#,
    )
    .bind(rates)
    .bind(now)
    .execute(pool)
    .await?;

    println!("DEBUG Currency rates updated at: {}", now);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Create table if not exists
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS currency_rates (
            id SERIAL PRIMARY KEY,
            rates JSONB NOT NULL,
            last_updated TIMESTAMPTZ NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // Run the initial update
    update_currency_rates(&pool).await?;

    let pool = Arc::new(pool);
    let mut scheduler = JobScheduler::new();

    // Schedule job to run daily at midnight UTC
    let pool_clone = Arc::clone(&pool);
    scheduler.add(Job::new("0 0 0 * * *".parse().unwrap(), move || {
        let pool = Arc::clone(&pool_clone);
        tokio::spawn(async move {
            if let Err(e) = update_currency_rates(&pool).await {
                eprintln!("Error updating currency rates: {}", e);
            }
        });
    }));

    println!("DEBUG Scheduler started, waiting for next job execution...");
    loop {
        scheduler.tick();
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}
