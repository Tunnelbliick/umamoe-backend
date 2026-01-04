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
        (date_trunc('month', CURRENT_TIMESTAMP AT TIME ZONE 'Asia/Tokyo') AT TIME ZONE 'Asia/Tokyo') AT TIME ZONE 'Europe/Berlin' as calculated_time,
        CURRENT_TIMESTAMP AT TIME ZONE 'Asia/Tokyo' as now_jst,
        date_trunc('month', CURRENT_TIMESTAMP AT TIME ZONE 'Asia/Tokyo') as start_month_jst_wall,
        (date_trunc('month', CURRENT_TIMESTAMP AT TIME ZONE 'Asia/Tokyo') AT TIME ZONE 'Asia/Tokyo') as start_month_jst_tz
    ";

    let row = sqlx::query(query).fetch_one(&pool).await?;
    
    let calculated: chrono::NaiveDateTime = row.get("calculated_time");
    let now_jst: chrono::NaiveDateTime = row.get("now_jst");
    
    println!("Now JST (Wall): {}", now_jst);
    println!("Calculated CET Threshold: {}", calculated);

    Ok(())
}
