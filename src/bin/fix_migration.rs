use sqlx::postgres::PgPoolOptions;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;

    println!("Connected to database. Deleting failed migration record...");

    sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 20251210000000")
        .execute(&pool)
        .await?;

    println!("Successfully deleted migration record 20251210000000.");
    
    // Also try to drop the indexes if they exist, to ensure clean slate
    println!("Dropping potentially half-created indexes...");
    let indexes = [
        "idx_inheritance_filter_composite",
        "idx_inheritance_main_chara_id_hash",
        "idx_support_card_experience_account",
        "idx_trainer_follower_account_composite"
    ];

    for idx in indexes {
        let sql = format!("DROP INDEX IF EXISTS {}", idx);
        sqlx::query(&sql).execute(&pool).await?;
        println!("Dropped {}", idx);
    }

    println!("Done. You can now run the server to apply the migration.");
    Ok(())
}
