use sqlx::postgres::PgPoolOptions;
use std::env;
use sqlx::Row;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .connect(&database_url)
        .await?;

    let query = "SELECT 
        ((date_trunc('month', CURRENT_TIMESTAMP AT TIME ZONE 'Asia/Tokyo') + interval '12 hours') AT TIME ZONE 'Asia/Tokyo') AT TIME ZONE 'Europe/Berlin' as calculated_threshold
    ";

    let row = sqlx::query(query).fetch_one(&pool).await?;
    
    let calculated: chrono::NaiveDateTime = row.get("calculated_threshold");
    
    println!("Calculated Threshold (12:00 PM JST -> CET): {}", calculated);

    Ok(())
}
