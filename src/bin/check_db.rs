use sqlx::postgres::PgPoolOptions;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .connect(&database_url)
        .await?;

    let row: (String,) = sqlx::query_as("SELECT data_type FROM information_schema.columns WHERE table_name = 'circles' AND column_name = 'last_updated'")
        .fetch_one(&pool)
        .await?;

    println!("last_updated type: {}", row.0);
    
    // Also check current time and timezone settings
    let time_check: (String, String) = sqlx::query_as("SELECT NOW()::text, current_setting('TIMEZONE')")
        .fetch_one(&pool)
        .await?;
        
    println!("DB Now: {}, Timezone: {}", time_check.0, time_check.1);

    Ok(())
}
