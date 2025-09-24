use axum::{
    extract::{Query, State},
    response::Json,
    routing::get,
    Router,
};
use sqlx::{Row, QueryBuilder, Postgres, Execute};
use tracing::info;

use crate::{
    errors::Result,
    models::{
        Inheritance, SupportCard, SearchResponse, UnifiedSearchParams, UnifiedAccountRecord,
    },
    AppState,
};

/// Creates efficient PostgreSQL array range queries for spark filtering
/// Uses PostgreSQL's && operator which is optimized for GIN indexes
fn add_spark_range_conditions<'a>(
    query_builder: &mut QueryBuilder<'a, Postgres>,
    column: &str,
    sparks: &'a [i32],
) {
    use std::collections::HashMap;
    
    let mut factor_groups: HashMap<i32, i32> = HashMap::new();
    
    // Group by factor ID and find minimum level for each factor
    for &spark in sparks {
        let factor_id = spark / 10;
        let level = spark % 10;
        factor_groups.entry(factor_id)
            .and_modify(|min_level| *min_level = (*min_level).min(level))
            .or_insert(level);
    }
    
    // Use && operator for each factor - this is the fastest approach for range queries
    for (factor_id, min_level) in factor_groups {
        // Build array of all valid values for this factor
        let values: Vec<i32> = (min_level..=9).map(|l| factor_id * 10 + l).collect();
        
        query_builder.push(" AND ");
        query_builder.push(column);
        query_builder.push(" && ARRAY[");
        
        for (i, val) in values.iter().enumerate() {
            if i > 0 {
                query_builder.push(",");
            }
            query_builder.push_bind(*val);
        }
        query_builder.push("]::int[]");
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/search", get(unified_search))
        .route("/count", get(get_unified_count))
}

pub async fn unified_search(
    State(state): State<AppState>,
    Query(params): Query<UnifiedSearchParams>,
) -> Result<Json<SearchResponse<UnifiedAccountRecord>>> {
    let page = params.page.unwrap_or(0);
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = page * limit;

    if std::env::var("DEBUG_MODE").unwrap_or_default() == "true" {
        info!("=== Unified Search Parameters ===");
        info!("Raw params: {:?}", params);
        info!("================================");
    }

    let total_count = execute_count_query(&state, &params).await?;
    let records = execute_search_query(&state, &params, limit, offset).await?;

    let total_pages = if limit > 0 {
        ((total_count as f64) / (limit as f64)).ceil() as i64
    } else {
        0
    };

    if std::env::var("DEBUG_MODE").unwrap_or_default() == "true" {
        info!("Found {} accounts (page {}, limit {})", records.len(), page, limit);
    }

    Ok(Json(SearchResponse {
        items: records,
        total: total_count,
        page,
        limit,
        total_pages,
    }))
}

async fn execute_search_query(
    state: &AppState,
    params: &UnifiedSearchParams,
    limit: i64,
    offset: i64,
) -> Result<Vec<UnifiedAccountRecord>> {
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
        SELECT 
            t.account_id,
            t.name as trainer_name,
            t.follower_num,
            t.last_updated,
            -- Inheritance fields (direct from JOIN)
            i.inheritance_id,
            i.main_parent_id,
            i.parent_left_id,
            i.parent_right_id,
            i.parent_rank,
            i.parent_rarity,
            i.blue_sparks,
            i.pink_sparks,
            i.green_sparks,
            i.white_sparks,
            i.win_count,
            i.white_count,
            i.main_blue_factors,
            i.main_pink_factors,
            i.main_green_factors,
            i.main_white_factors,
            i.main_white_count,
            -- Support card fields (direct from JOIN)
            sc.support_card_id,
            sc.limit_break_count,
            sc.experience
        FROM trainer t
        INNER JOIN inheritance i ON t.account_id = i.account_id
        INNER JOIN support_card sc ON t.account_id = sc.account_id
        WHERE (t.follower_num IS NULL OR t.follower_num < 1000)
        "#
    );

    // Filter based on search type
    match params.search_type.as_deref() {
        Some("inheritance") => {
            query_builder.push(" AND i.inheritance_id IS NOT NULL");
        }
        Some("support_cards") => {
            query_builder.push(" AND sc.support_card_id IS NOT NULL");
        }
        _ => {
            // Default: accounts that have either inheritance OR support cards
            query_builder.push(" AND (i.inheritance_id IS NOT NULL OR sc.support_card_id IS NOT NULL)");
        }
    }

    // Apply inheritance filters directly (no EXISTS needed)
    if let Some(trainer_id) = &params.trainer_id {
        query_builder.push(" AND t.account_id = ");
        query_builder.push_bind(trainer_id);
    }
    
    if let Some(main_parent_id) = params.main_parent_id {
        query_builder.push(" AND i.main_parent_id = ");
        query_builder.push_bind(main_parent_id);
    }
    
    if let Some(parent_left_id) = params.parent_left_id {
        query_builder.push(" AND i.parent_left_id = ");
        query_builder.push_bind(parent_left_id);
    }
    
    if let Some(parent_right_id) = params.parent_right_id {
        query_builder.push(" AND i.parent_right_id = ");
        query_builder.push_bind(parent_right_id);
    }
    
    if let Some(parent_rank) = params.parent_rank {
        query_builder.push(" AND i.parent_rank >= ");
        query_builder.push_bind(parent_rank);
    }
    
    if let Some(parent_rarity) = params.parent_rarity {
        query_builder.push(" AND i.parent_rarity >= ");
        query_builder.push_bind(parent_rarity);
    }
    
    // Add spark filters
    if let Some(blue_sparks) = &params.blue_sparks {
        if !blue_sparks.is_empty() {
            add_spark_range_conditions(&mut query_builder, "i.blue_sparks", blue_sparks);
        }
    }
    
    if let Some(pink_sparks) = &params.pink_sparks {
        if !pink_sparks.is_empty() {
            add_spark_range_conditions(&mut query_builder, "i.pink_sparks", pink_sparks);
        }
    }
    
    if let Some(green_sparks) = &params.green_sparks {
        if !green_sparks.is_empty() {
            add_spark_range_conditions(&mut query_builder, "i.green_sparks", green_sparks);
        }
    }
    
    if let Some(white_sparks) = &params.white_sparks {
        if !white_sparks.is_empty() {
            add_spark_range_conditions(&mut query_builder, "i.white_sparks", white_sparks);
        }
    }
    
    if let Some(min_win_count) = params.min_win_count {
        query_builder.push(" AND i.win_count >= ");
        query_builder.push_bind(min_win_count);
    }
    
    if let Some(min_white_count) = params.min_white_count {
        query_builder.push(" AND i.white_count >= ");
        query_builder.push_bind(min_white_count);
    }
    
    // Main inherit filtering
    if let Some(min_main_blue) = params.min_main_blue_factors {
        query_builder.push(" AND i.main_blue_factors >= ");
        query_builder.push_bind(min_main_blue);
    }
    
    if let Some(min_main_pink) = params.min_main_pink_factors {
        query_builder.push(" AND i.main_pink_factors >= ");
        query_builder.push_bind(min_main_pink);
    }
    
    if let Some(min_main_green) = params.min_main_green_factors {
        query_builder.push(" AND i.main_green_factors >= ");
        query_builder.push_bind(min_main_green);
    }
    
    if let Some(main_white_factors) = &params.main_white_factors {
        if !main_white_factors.is_empty() {
            add_spark_range_conditions(&mut query_builder, "i.main_white_factors", main_white_factors);
        }
    }
    
    if let Some(min_main_white_count) = params.min_main_white_count {
        query_builder.push(" AND i.main_white_count >= ");
        query_builder.push_bind(min_main_white_count);
    }

    // Support card filters - direct WHERE conditions (no EXISTS needed)
    if let Some(support_card_id) = params.support_card_id {
        query_builder.push(" AND sc.support_card_id = ");
        query_builder.push_bind(support_card_id);
    }
    
    if let Some(min_limit_break) = params.min_limit_break {
        query_builder.push(" AND sc.limit_break_count >= ");
        query_builder.push_bind(min_limit_break);
    }
    
    if let Some(max_limit_break) = params.max_limit_break {
        query_builder.push(" AND sc.limit_break_count <= ");
        query_builder.push_bind(max_limit_break);
    }
    
    if let Some(min_experience) = params.min_experience {
        query_builder.push(" AND sc.experience >= ");
        query_builder.push_bind(min_experience);
    }

    if let Some(max_follower_num) = params.max_follower_num {
        query_builder.push(" AND (t.follower_num IS NULL OR t.follower_num <= ");
        query_builder.push_bind(max_follower_num);
        query_builder.push(")");
    }

    // Simplified ordering - no subqueries needed
    let order_by_clause = match params.sort_by.as_deref() {
        Some("win_count") => " ORDER BY i.win_count DESC, t.account_id ASC",
        Some("white_count") => " ORDER BY i.white_count DESC, t.account_id ASC",
        Some("parent_rank") => " ORDER BY i.parent_rank DESC, t.account_id ASC",
        Some("submitted_at") | Some("last_updated") => " ORDER BY t.last_updated DESC, t.account_id ASC",
        Some("main_blue_factors") => " ORDER BY i.main_blue_factors DESC, t.account_id ASC",
        Some("main_pink_factors") => " ORDER BY i.main_pink_factors DESC, t.account_id ASC",
        Some("main_green_factors") => " ORDER BY i.main_green_factors DESC, t.account_id ASC",
        Some("main_white_count") => " ORDER BY i.main_white_count DESC, t.account_id ASC",
        Some("experience") => " ORDER BY sc.experience DESC NULLS LAST, t.account_id ASC",
        Some("limit_break_count") => " ORDER BY sc.limit_break_count DESC NULLS LAST, t.account_id ASC",
        Some("follower_num") => " ORDER BY COALESCE(t.follower_num, 999999) ASC, t.account_id ASC",
        _ => {
            // Default ordering based on search type
            match params.search_type.as_deref() {
                Some("support_cards") => {
                    " ORDER BY sc.experience DESC NULLS LAST, sc.limit_break_count DESC NULLS LAST, t.account_id ASC"
                },
                Some("inheritance") => {
                    " ORDER BY COALESCE(t.follower_num, 0) ASC, i.parent_rank DESC, i.win_count DESC, t.account_id ASC"
                },
                _ => {
                    " ORDER BY COALESCE(t.follower_num, 0) ASC, i.parent_rank DESC NULLS LAST, i.win_count DESC NULLS LAST, t.account_id ASC"
                }
            }
        }
    };
    
    query_builder.push(order_by_clause);
    query_builder.push(" LIMIT ");
    query_builder.push_bind(limit);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let query = query_builder.build();
    
    if std::env::var("DEBUG_MODE").unwrap_or_default() == "true" {
        info!("Search query SQL: {}", query.sql());
    }

    let rows = query.fetch_all(&state.db).await?;

    if std::env::var("DEBUG_MODE").unwrap_or_default() == "true" {
        info!("Query returned {} rows", rows.len());
    }

    let mut records = Vec::new();
    for row in rows {
        let account_id: String = row.get("account_id");
        
        // Build support card directly from row (no JSON parsing needed)
        let support_card: Option<SupportCard> = if row.try_get::<Option<i32>, _>("support_card_id")?.is_some() {
            Some(SupportCard {
                account_id: account_id.clone(),
                support_card_id: row.get("support_card_id"),
                limit_break_count: row.get("limit_break_count"),
                experience: row.get("experience"),
            })
        } else {
            None
        };

        // Build inheritance object if it exists
        let inheritance: Option<Inheritance> = if row.try_get::<Option<i32>, _>("inheritance_id")?.is_some() {
            Some(Inheritance {
                inheritance_id: row.get("inheritance_id"),
                account_id: account_id.clone(),
                main_parent_id: row.get("main_parent_id"),
                parent_left_id: row.get("parent_left_id"),
                parent_right_id: row.get("parent_right_id"),
                parent_rank: row.get("parent_rank"),
                parent_rarity: row.get("parent_rarity"),
                blue_sparks: row.get("blue_sparks"),
                pink_sparks: row.get("pink_sparks"),
                green_sparks: row.get("green_sparks"),
                white_sparks: row.get("white_sparks"),
                win_count: row.get("win_count"),
                white_count: row.get("white_count"),
                main_blue_factors: row.get("main_blue_factors"),
                main_pink_factors: row.get("main_pink_factors"),
                main_green_factors: row.get("main_green_factors"),
                main_white_factors: row.get("main_white_factors"),
                main_white_count: row.get("main_white_count"),
            })
        } else {
            None
        };

        records.push(UnifiedAccountRecord {
            account_id,
            trainer_name: row.get("trainer_name"),
            follower_num: row.get("follower_num"),
            last_updated: row.get("last_updated"),
            inheritance,
            support_card,
        });
    }

    Ok(records)
}

async fn execute_count_query(
    state: &AppState,
    params: &UnifiedSearchParams,
) -> Result<i64> {
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
        SELECT COUNT(*) 
        FROM trainer t
        INNER JOIN inheritance i ON t.account_id = i.account_id
        INNER JOIN support_card sc ON t.account_id = sc.account_id
        WHERE (t.follower_num IS NULL OR t.follower_num < 1000)
        "#
    );

    // Filter based on search type
    match params.search_type.as_deref() {
        Some("inheritance") => {
            query_builder.push(" AND i.inheritance_id IS NOT NULL");
        }
        Some("support_cards") => {
            query_builder.push(" AND sc.support_card_id IS NOT NULL");
        }
        _ => {
            query_builder.push(" AND (i.inheritance_id IS NOT NULL OR sc.support_card_id IS NOT NULL)");
        }
    }

    // Apply inheritance filters directly
    if let Some(trainer_id) = &params.trainer_id {
        query_builder.push(" AND t.account_id = ");
        query_builder.push_bind(trainer_id);
    }
    
    if let Some(main_parent_id) = params.main_parent_id {
        query_builder.push(" AND i.main_parent_id = ");
        query_builder.push_bind(main_parent_id);
    }
    
    if let Some(parent_left_id) = params.parent_left_id {
        query_builder.push(" AND i.parent_left_id = ");
        query_builder.push_bind(parent_left_id);
    }
    
    if let Some(parent_right_id) = params.parent_right_id {
        query_builder.push(" AND i.parent_right_id = ");
        query_builder.push_bind(parent_right_id);
    }
    
    if let Some(parent_rank) = params.parent_rank {
        query_builder.push(" AND i.parent_rank >= ");
        query_builder.push_bind(parent_rank);
    }
    
    if let Some(parent_rarity) = params.parent_rarity {
        query_builder.push(" AND i.parent_rarity >= ");
        query_builder.push_bind(parent_rarity);
    }
    
    // Add spark filters
    if let Some(blue_sparks) = &params.blue_sparks {
        if !blue_sparks.is_empty() {
            add_spark_range_conditions(&mut query_builder, "i.blue_sparks", blue_sparks);
        }
    }
    
    if let Some(pink_sparks) = &params.pink_sparks {
        if !pink_sparks.is_empty() {
            add_spark_range_conditions(&mut query_builder, "i.pink_sparks", pink_sparks);
        }
    }
    
    if let Some(green_sparks) = &params.green_sparks {
        if !green_sparks.is_empty() {
            add_spark_range_conditions(&mut query_builder, "i.green_sparks", green_sparks);
        }
    }
    
    if let Some(white_sparks) = &params.white_sparks {
        if !white_sparks.is_empty() {
            add_spark_range_conditions(&mut query_builder, "i.white_sparks", white_sparks);
        }
    }
    
    if let Some(min_win_count) = params.min_win_count {
        query_builder.push(" AND i.win_count >= ");
        query_builder.push_bind(min_win_count);
    }
    
    if let Some(min_white_count) = params.min_white_count {
        query_builder.push(" AND i.white_count >= ");
        query_builder.push_bind(min_white_count);
    }
    
    // Main inherit filtering
    if let Some(min_main_blue) = params.min_main_blue_factors {
        query_builder.push(" AND i.main_blue_factors >= ");
        query_builder.push_bind(min_main_blue);
    }
    
    if let Some(min_main_pink) = params.min_main_pink_factors {
        query_builder.push(" AND i.main_pink_factors >= ");
        query_builder.push_bind(min_main_pink);
    }
    
    if let Some(min_main_green) = params.min_main_green_factors {
        query_builder.push(" AND i.main_green_factors >= ");
        query_builder.push_bind(min_main_green);
    }
    
    if let Some(main_white_factors) = &params.main_white_factors {
        if !main_white_factors.is_empty() {
            add_spark_range_conditions(&mut query_builder, "i.main_white_factors", main_white_factors);
        }
    }
    
    if let Some(min_main_white_count) = params.min_main_white_count {
        query_builder.push(" AND i.main_white_count >= ");
        query_builder.push_bind(min_main_white_count);
    }

    // Support card filters - direct WHERE conditions
    if let Some(support_card_id) = params.support_card_id {
        query_builder.push(" AND sc.support_card_id = ");
        query_builder.push_bind(support_card_id);
    }
    
    if let Some(min_limit_break) = params.min_limit_break {
        query_builder.push(" AND sc.limit_break_count >= ");
        query_builder.push_bind(min_limit_break);
    }
    
    if let Some(max_limit_break) = params.max_limit_break {
        query_builder.push(" AND sc.limit_break_count <= ");
        query_builder.push_bind(max_limit_break);
    }
    
    if let Some(min_experience) = params.min_experience {
        query_builder.push(" AND sc.experience >= ");
        query_builder.push_bind(min_experience);
    }

    if let Some(max_follower_num) = params.max_follower_num {
        query_builder.push(" AND (t.follower_num IS NULL OR t.follower_num <= ");
        query_builder.push_bind(max_follower_num);
        query_builder.push(")");
    }

    let query = query_builder.build();
    
    if std::env::var("DEBUG_MODE").unwrap_or_default() == "true" {
        info!("Count query SQL: {}", query.sql());
    }

    let row = query.fetch_one(&state.db).await?;
    Ok(row.get::<i64, _>(0))
}

pub async fn get_unified_count(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>> {
    let total_inheritance_count = sqlx::query("SELECT COUNT(*) FROM inheritance")
        .fetch_one(&state.db)
        .await?
        .get::<i64, _>(0);

    let total_support_card_accounts = sqlx::query("SELECT COUNT(DISTINCT account_id) FROM support_card")
        .fetch_one(&state.db)
        .await?
        .get::<i64, _>(0);

    let available_inheritance_count = sqlx::query(
        r#"
        SELECT COUNT(*) 
        FROM inheritance i 
        INNER JOIN trainer t ON i.account_id = t.account_id
        WHERE (t.follower_num IS NULL OR t.follower_num < 1000)
        "#
    )
    .fetch_one(&state.db)
    .await?
    .get::<i64, _>(0);

    let available_support_card_accounts = sqlx::query(
        r#"
        SELECT COUNT(DISTINCT sc.account_id) 
        FROM support_card sc 
        INNER JOIN trainer t ON sc.account_id = t.account_id
        WHERE (t.follower_num IS NULL OR t.follower_num < 1000)
        "#
    )
    .fetch_one(&state.db)
    .await?
    .get::<i64, _>(0);

    Ok(Json(serde_json::json!({
        "total_inheritance_records": total_inheritance_count,
        "total_support_card_accounts": total_support_card_accounts,
        "available_inheritance_records": available_inheritance_count,
        "available_support_card_accounts": available_support_card_accounts,
    })))
}
