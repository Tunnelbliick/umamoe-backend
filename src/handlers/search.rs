use axum::{
    extract::State,
    response::Json,
    routing::get,
    Router,
};
use sqlx::{Postgres, QueryBuilder, Row};

use crate::{
    errors::Result,
    models::{Inheritance, SearchResponse, SupportCard, UnifiedAccountRecord, UnifiedSearchParams},
    AppState,
};

fn get_affinity_expression(player_chara_id: Option<i32>) -> String {
    match player_chara_id {
        None => "(COALESCE(i.base_affinity, 0) + COALESCE(i.race_affinity, 0))".to_string(),
        Some(p_val) => {
            let chara_id = if p_val > 100000 { p_val / 100 } else { p_val };
            let array_index = chara_id - 1000;
            format!(
                "(COALESCE(i.affinity_scores[{}], 0) + COALESCE(i.race_affinity, 0))",
                array_index
            )
        }
    }
}

fn add_main_parent_spark_conditions<'a>(
    query_builder: &mut QueryBuilder<'a, Postgres>,
    column: &str,
    sparks: &[i32],
) {
    if sparks.is_empty() {
        return;
    }

    let mut specific_sparks = Vec::new();
    let mut wildcard_levels = Vec::new();

    for &spark in sparks {
        if spark >= 10 {
            specific_sparks.push(spark);
        } else {
            wildcard_levels.push(spark);
        }
    }

    query_builder.push(" AND (");
    let mut has_condition = false;

    if !specific_sparks.is_empty() {
        has_condition = true;
        query_builder.push(column);
        query_builder.push(" = ANY(ARRAY[");
        for (i, val) in specific_sparks.iter().enumerate() {
            if i > 0 {
                query_builder.push(",");
            }
            query_builder.push_bind(*val);
        }
        query_builder.push("]::int[])");
    }

    if !wildcard_levels.is_empty() {
        if has_condition {
            query_builder.push(" OR ");
        }
        let min_wildcard = wildcard_levels.iter().min().unwrap();
        query_builder.push(format!("({} % 10 >= {})", column, min_wildcard));
    }

    query_builder.push(")");
}

fn add_spark_range_conditions<'a>(
    query_builder: &mut QueryBuilder<'a, Postgres>,
    column: &str,
    sparks: &[i32],
) {
    if sparks.is_empty() {
        return;
    }

    let mut specific_sparks = Vec::new();
    let mut wildcard_levels = Vec::new();

    for &spark in sparks {
        if spark >= 10 {
            specific_sparks.push(spark);
        } else {
            wildcard_levels.push(spark);
        }
    }

    query_builder.push(" AND (");
    let mut has_condition = false;

    if !specific_sparks.is_empty() {
        has_condition = true;
        query_builder.push(column);
        query_builder.push(" && ARRAY[");
        for (i, val) in specific_sparks.iter().enumerate() {
            if i > 0 {
                query_builder.push(",");
            }
            query_builder.push_bind(*val);
        }
        query_builder.push("]::int[]");
    }

    if !wildcard_levels.is_empty() {
        if has_condition {
            query_builder.push(" OR ");
        }
        let max_factor_id = 100;
        let mut all_possible_sparks = Vec::new();
        for factor_id in 1..=max_factor_id {
            for &level in &wildcard_levels {
                all_possible_sparks.push(factor_id * 10 + level);
            }
        }
        query_builder.push(column);
        query_builder.push(" && ARRAY[");
        for (i, val) in all_possible_sparks.iter().enumerate() {
            if i > 0 {
                query_builder.push(",");
            }
            query_builder.push_bind(*val);
        }
        query_builder.push("]::int[]");
    }

    query_builder.push(")");
}

fn add_9star_spark_conditions<'a>(
    query_builder: &mut QueryBuilder<'a, Postgres>,
    column: &str,
    desired_star: i32,
) {
    let values: Vec<i32> = (1..=6)
        .map(|stat_type| stat_type * 100 + desired_star)
        .collect();

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

fn process_spark_groups(groups: &[String]) -> Vec<Vec<i32>> {
    groups.iter()
        .map(|s| s.split(',').filter_map(|v| v.trim().parse::<i32>().ok()).collect::<Vec<i32>>())
        .filter(|v| !v.is_empty())
        .collect()
}

fn add_multi_group_spark_conditions<'a>(
    query_builder: &mut QueryBuilder<'a, Postgres>,
    column: &str,
    groups: &[Vec<i32>],
) {
    if groups.is_empty() {
        return;
    }

    if groups.len() == 1 {
        add_spark_range_conditions(query_builder, column, &groups[0]);
        return;
    }

    let mut group_values: Vec<Vec<i32>> = Vec::new();
    for group in groups {
        let values = expand_spark_group(group);
        group_values.push(values);
    }

    let n = groups.len();
    
    // Check if all groups are identical (e.g., 3x "Any 3*")
    let all_groups_identical = group_values.windows(2).all(|w| {
        let set1: std::collections::HashSet<i32> = w[0].iter().copied().collect();
        let set2: std::collections::HashSet<i32> = w[1].iter().copied().collect();
        set1 == set2
    });

    if all_groups_identical && !group_values.is_empty() {
        // All groups are the same - we need to count how many elements match
        // Use array intersection and check cardinality
        query_builder.push(" AND cardinality(ARRAY(SELECT unnest(");
        query_builder.push(column);
        query_builder.push(") INTERSECT SELECT unnest(ARRAY[");
        for (i, val) in group_values[0].iter().enumerate() {
            if i > 0 { query_builder.push(","); }
            query_builder.push_bind(*val);
        }
        query_builder.push("]::int[]))) >= ");
        query_builder.push_bind(n as i32);
        return;
    }
    
    if n == 2 {
        let set1: std::collections::HashSet<i32> = group_values[0].iter().copied().collect();
        let set2: std::collections::HashSet<i32> = group_values[1].iter().copied().collect();
        let groups_are_disjoint = set1.is_disjoint(&set2);
        
        if groups_are_disjoint {
            query_builder.push(" AND (");
            query_builder.push(column);
            query_builder.push(" && ARRAY[");
            for (i, val) in group_values[0].iter().enumerate() {
                if i > 0 { query_builder.push(","); }
                query_builder.push_bind(*val);
            }
            query_builder.push("]::int[]) AND (");
            query_builder.push(column);
            query_builder.push(" && ARRAY[");
            for (i, val) in group_values[1].iter().enumerate() {
                if i > 0 { query_builder.push(","); }
                query_builder.push_bind(*val);
            }
            query_builder.push("]::int[])");
        } else {
            // Overlapping groups - need to count matches in each group
            query_builder.push(" AND cardinality(ARRAY(SELECT unnest(");
            query_builder.push(column);
            query_builder.push(") INTERSECT SELECT unnest(ARRAY[");
            for (i, val) in group_values[0].iter().enumerate() {
                if i > 0 { query_builder.push(","); }
                query_builder.push_bind(*val);
            }
            query_builder.push("]::int[]))) >= 1 AND cardinality(ARRAY(SELECT unnest(");
            query_builder.push(column);
            query_builder.push(") INTERSECT SELECT unnest(ARRAY[");
            for (i, val) in group_values[1].iter().enumerate() {
                if i > 0 { query_builder.push(","); }
                query_builder.push_bind(*val);
            }
            query_builder.push("]::int[]))) >= 1 AND cardinality(");
            query_builder.push(column);
            query_builder.push(") >= 2");
        }
    } else {
        // For 3+ groups, check if they're all disjoint
        let mut all_disjoint = true;
        for i in 0..group_values.len() {
            for j in (i+1)..group_values.len() {
                let set1: std::collections::HashSet<i32> = group_values[i].iter().copied().collect();
                let set2: std::collections::HashSet<i32> = group_values[j].iter().copied().collect();
                if !set1.is_disjoint(&set2) {
                    all_disjoint = false;
                    break;
                }
            }
            if !all_disjoint { break; }
        }

        if all_disjoint {
            // All groups are disjoint - simple overlap check for each
            query_builder.push(" AND (");
            for (idx, values) in group_values.iter().enumerate() {
                if idx > 0 { query_builder.push(" AND "); }
                query_builder.push(column);
                query_builder.push(" && ARRAY[");
                for (i, val) in values.iter().enumerate() {
                    if i > 0 { query_builder.push(","); }
                    query_builder.push_bind(*val);
                }
                query_builder.push("]::int[]");
            }
            query_builder.push(")");
        } else {
            // Some groups overlap - combine all values and count matches
            let mut all_values: Vec<i32> = group_values.iter().flatten().copied().collect();
            all_values.sort();
            all_values.dedup();
            
            query_builder.push(" AND cardinality(ARRAY(SELECT unnest(");
            query_builder.push(column);
            query_builder.push(") INTERSECT SELECT unnest(ARRAY[");
            for (i, val) in all_values.iter().enumerate() {
                if i > 0 { query_builder.push(","); }
                query_builder.push_bind(*val);
            }
            query_builder.push("]::int[]))) >= ");
            query_builder.push_bind(n as i32);
        }
    }
}

fn expand_spark_group(sparks: &[i32]) -> Vec<i32> {
    let mut result = Vec::new();
    let mut wildcard_levels = Vec::new();
    
    for &spark in sparks {
        if spark >= 10 {
            result.push(spark);
        } else {
            wildcard_levels.push(spark);
        }
    }

    if !wildcard_levels.is_empty() {
        let max_factor_id = 100;
        for factor_id in 1..=max_factor_id {
            for &level in &wildcard_levels {
                result.push(factor_id * 10 + level);
            }
        }
    }
    
    result.sort();
    result.dedup();
    result
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/search", get(unified_search))
        .route("/count", get(get_unified_count))
}

fn parse_search_params(query: &str) -> UnifiedSearchParams {
    let mut params_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for (k, v) in url::form_urlencoded::parse(query.as_bytes()) {
        params_map.entry(k.to_string()).or_default().push(v.to_string());
    }

    let get_i32 = |key: &str| -> Option<i32> {
        params_map.get(key).and_then(|v| v.last()).and_then(|s| s.parse().ok())
    };
    
    let get_i64 = |key: &str| -> Option<i64> {
        params_map.get(key).and_then(|v| v.last()).and_then(|s| s.parse().ok())
    };

    let get_bool = |key: &str| -> Option<bool> {
        params_map.get(key).and_then(|v| v.last()).and_then(|s| s.parse().ok())
    };

    let get_string = |key: &str| -> Option<String> {
        params_map.get(key).and_then(|v| v.last()).cloned()
    };

    let get_vec = |key: &str| -> Vec<String> {
        params_map.get(key).cloned().unwrap_or_default()
    };

    UnifiedSearchParams {
        page: get_i64("page"),
        limit: get_i64("limit"),
        search_type: get_string("search_type"),
        main_parent_id: get_i32("main_parent_id"),
        parent_left_id: get_i32("parent_left_id"),
        parent_right_id: get_i32("parent_right_id"),
        parent_rank: get_i32("parent_rank"),
        parent_rarity: get_i32("parent_rarity"),
        blue_sparks: get_vec("blue_sparks"),
        pink_sparks: get_vec("pink_sparks"),
        green_sparks: get_vec("green_sparks"),
        white_sparks: get_vec("white_sparks"),
        blue_sparks_9star: get_bool("blue_sparks_9star"),
        pink_sparks_9star: get_bool("pink_sparks_9star"),
        green_sparks_9star: get_bool("green_sparks_9star"),
        main_parent_blue_sparks: get_vec("main_parent_blue_sparks"),
        main_parent_pink_sparks: get_vec("main_parent_pink_sparks"),
        main_parent_green_sparks: get_vec("main_parent_green_sparks"),
        main_parent_white_sparks: get_vec("main_parent_white_sparks"),
        min_win_count: get_i32("min_win_count"),
        min_white_count: get_i32("min_white_count"),
        min_main_blue_factors: get_i32("min_main_blue_factors"),
        min_main_pink_factors: get_i32("min_main_pink_factors"),
        min_main_green_factors: get_i32("min_main_green_factors"),
        main_white_factors: get_vec("main_white_factors"),
        min_main_white_count: get_i32("min_main_white_count"),
        optional_white_sparks: get_vec("optional_white_sparks"),
        optional_main_white_factors: {
            let v = get_vec("optional_main_white_factors");
            if v.is_empty() {
                get_vec("optional_main_white_sparks")
            } else {
                v
            }
        },
        support_card_id: get_i32("support_card_id"),
        min_limit_break: get_i32("min_limit_break"),
        max_limit_break: get_i32("max_limit_break"),
        min_experience: get_i32("min_experience"),
        trainer_id: get_string("trainer_id"),
        trainer_name: get_string("trainer_name"),
        max_follower_num: get_i32("max_follower_num"),
        sort_by: get_string("sort_by"),
        sort_order: get_string("sort_order"),
        player_chara_id: get_i32("player_chara_id"),
        player_chara_id_2: get_i32("player_chara_id_2"),
        desired_main_chara_id: get_i32("desired_main_chara_id"),
    }
}

pub async fn unified_search(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Result<Json<SearchResponse<UnifiedAccountRecord>>> {
    let query_string = request.uri().query().unwrap_or("");
    let params = parse_search_params(query_string);

    tracing::info!("🔍 SEARCH REQUEST: page={:?}, limit={:?}, search_type={:?}, sort_by={:?}, player_chara_id={:?}, filters={:?}", 
        params.page, params.limit, params.search_type, params.sort_by, params.player_chara_id,
        format!("{:?}", params).chars().take(200).collect::<String>());

    let page = params.page.unwrap_or(0);
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = page * limit;

    // Check if this is a blank/default query (no filters applied except search_type and sort)
    let is_blank_query = params.trainer_id.is_none()
        && params.trainer_name.is_none()
        && params.main_parent_id.is_none()
        && params.parent_left_id.is_none()
        && params.parent_right_id.is_none()
        && (params.parent_rank.is_none() || params.parent_rank == Some(1))
        && params.parent_rarity.is_none()
        && params.blue_sparks.is_empty()
        && params.pink_sparks.is_empty()
        && params.green_sparks.is_empty()
        && params.white_sparks.is_empty()
        && params.blue_sparks_9star.is_none()
        && params.pink_sparks_9star.is_none()
        && params.green_sparks_9star.is_none()
        && params.main_parent_blue_sparks.is_empty()
        && params.main_parent_pink_sparks.is_empty()
        && params.main_parent_green_sparks.is_empty()
        && params.main_parent_white_sparks.is_empty()
        && params.support_card_id.is_none()
        && params.min_limit_break.is_none()
        && params.max_limit_break.is_none()
        && params.min_experience.is_none()
        && (params.min_win_count.is_none() || params.min_win_count == Some(0))
        && (params.min_white_count.is_none() || params.min_white_count == Some(0))
        && params.min_main_blue_factors.is_none()
        && params.min_main_pink_factors.is_none()
        && params.min_main_green_factors.is_none()
        && params.main_white_factors.is_empty()
        && params.optional_white_sparks.is_empty()
        && params.optional_main_white_factors.is_empty()
        && (params.min_main_white_count.is_none() || params.min_main_white_count == Some(0))
        && params.desired_main_chara_id.is_none()
        && params.player_chara_id.is_none()
        && (params.max_follower_num.is_none() || params.max_follower_num == Some(1000) || params.max_follower_num == Some(999));

    // Cache only blank queries (materialized view makes these instant anyway)
    if is_blank_query {
        let search_type = params.search_type.as_deref().unwrap_or("all");
        let sort_by = params.sort_by.as_deref().unwrap_or("default");
        let cache_key = format!(
            "search:blank:{}:{}:page{}:limit{}",
            search_type, sort_by, page, limit
        );
        if let Some(cached) = crate::cache::get::<SearchResponse<UnifiedAccountRecord>>(&cache_key)
        {
            tracing::info!(
                "🎯 CACHE HIT: search - type={}, sort={}, page={}, limit={}",
                search_type,
                sort_by,
                page,
                limit
            );
            return Ok(Json(cached));
        }
        tracing::info!(
            "❌ CACHE MISS: search - type={}, sort={}, page={}, limit={}",
            search_type,
            sort_by,
            page,
            limit
        );
    }

    let query_start = std::time::Instant::now();
    let total_count = execute_count_query(&state, &params).await?;
    let count_duration = query_start.elapsed();
    tracing::info!("⏱️  COUNT QUERY: {}ms", count_duration.as_millis());

    let search_start = std::time::Instant::now();
    let records = execute_search_query(&state, &params, limit, offset).await?;
    let search_duration = search_start.elapsed();
    tracing::info!(
        "⏱️  SEARCH QUERY: {}ms (returned {} records) - player_chara_id={:?}",
        search_duration.as_millis(),
        records.len(),
        params.player_chara_id
    );

    let total_pages = if limit > 0 {
        ((total_count as f64) / (limit as f64)).ceil() as i64
    } else {
        0
    };

    let total_display = if !is_blank_query && total_count > 10000 {
        "over 10000".to_string()
    } else {
        total_count.to_string()
    };

    let response = SearchResponse {
        items: records,
        total: total_display,
        page,
        limit,
        total_pages,
    };

    // Cache blank queries for 1 hour (materialized view makes these instant)
    if is_blank_query {
        let search_type = params.search_type.as_deref().unwrap_or("all");
        let sort_by = params.sort_by.as_deref().unwrap_or("default");
        let cache_key = format!(
            "search:blank:{}:{}:page{}:limit{}",
            search_type, sort_by, page, limit
        );
        if crate::cache::set(&cache_key, &response, std::time::Duration::from_secs(3600)).is_ok() {
            tracing::info!(
                "💾 CACHE SET: search - type={}, sort={}, page={}, limit={}",
                search_type,
                sort_by,
                page,
                limit
            );
        }
    }

    tracing::info!(
        "✅ SEARCH COMPLETE: returned {} items, total={}, page={}, total_pages={}",
        response.items.len(),
        response.total,
        response.page,
        response.total_pages
    );

    Ok(Json(response))
}

async fn execute_search_query(
    state: &AppState,
    params: &UnifiedSearchParams,
    limit: i64,
    offset: i64,
) -> Result<Vec<UnifiedAccountRecord>> {
    // eprintln!(
    //     "🚀 execute_search_query START - player_chara_id={:?}",
    //     params.player_chara_id
    // );
    // tracing::info!("🔍 UNIFIED SEARCH: Inheritance-first with support card join");

    // Build unified query: always start from inheritance, join support card
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new("");
    
    // Use desired_main_chara_id for affinity calculation if provided, otherwise use player_chara_id
    // This allows filtering by main character AND calculating affinity for that character
    let affinity_player_id = params.desired_main_chara_id.or(params.player_chara_id);
    let affinity_expr = get_affinity_expression(affinity_player_id);

    query_builder.push(
        r#"
        SELECT
            i.account_id,
            t.name as trainer_name,
            t.follower_num,
            t.last_updated,
            -- Inheritance fields
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
            -- Affinity score calculation
            ("#,
    );
    query_builder.push(&affinity_expr);
    query_builder.push(r#") as affinity_score"#);

    // Parse optional white spark factor IDs for scoring
    // Handle both comma-separated single string and multiple params
    let optional_white_sparks_ids: Vec<i32> = params.optional_white_sparks
        .iter()
        .flat_map(|s| s.split(','))
        .filter_map(|v| v.trim().parse::<i32>().ok())
        .collect();
    let optional_main_white_factors_ids: Vec<i32> = params.optional_main_white_factors
        .iter()
        .flat_map(|s| s.split(','))
        .filter_map(|v| v.trim().parse::<i32>().ok())
        .collect();

    // Log what we're scoring
    if !optional_white_sparks_ids.is_empty() || !optional_main_white_factors_ids.is_empty() {
        tracing::info!("🎯 OPTIONAL SCORING: white_sparks_ids={:?}, main_white_factors_ids={:?}", 
            optional_white_sparks_ids, optional_main_white_factors_ids);
    }

    // Add white_sparks scoring column - ONLY score factors requested for white_sparks
    if !optional_white_sparks_ids.is_empty() {
        query_builder.push(", calculate_sparks_score(i.white_sparks, ARRAY[");
        for (i, val) in optional_white_sparks_ids.iter().enumerate() {
            if i > 0 { query_builder.push(","); }
            query_builder.push_bind(*val);
        }
        query_builder.push("]::int[]) AS white_sparks_score");
    } else {
        query_builder.push(", 0 AS white_sparks_score");
    }

    // Add main_white_factors scoring column - ONLY score factors requested for main_white_factors
    if !optional_main_white_factors_ids.is_empty() {
        query_builder.push(", calculate_sparks_score(i.main_white_factors, ARRAY[");
        for (i, val) in optional_main_white_factors_ids.iter().enumerate() {
            if i > 0 { query_builder.push(","); }
            query_builder.push_bind(*val);
        }
        query_builder.push("]::int[]) AS main_white_factors_score");
    } else {
        query_builder.push(", 0 AS main_white_factors_score");
    }

    query_builder.push(
        r#",
            -- Support card fields (best one per account)
            sc.support_card_id,
            sc.limit_break_count,
            sc.experience
        FROM inheritance i
        INNER JOIN trainer t ON i.account_id = t.account_id
        LEFT JOIN support_card sc ON i.account_id = sc.account_id
        WHERE 1=1
    "#,
    );

    // Support card filters
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

    // Follower filter
    query_builder.push(" AND (t.follower_num IS NULL OR t.follower_num < 1000)");

    // Player exclusion - don't show inheritances where player is the main character
    // Use the same player ID as affinity calculation (desired_main_chara_id takes precedence)
    let affinity_player_id = params.desired_main_chara_id.or(params.player_chara_id);
    if let Some(player_id) = affinity_player_id {
        // Only exclude if we're NOT filtering for this specific character as main parent
        // (when desired_main_chara_id is set, we WANT that character as main parent)
        if params.desired_main_chara_id.is_none() {
            query_builder.push(" AND i.main_chara_id != ");
            query_builder.push_bind(player_id);
        }
    }

    // Apply inheritance filters directly (no EXISTS needed)
    if let Some(trainer_id) = &params.trainer_id {
        query_builder.push(" AND t.account_id = ");
        query_builder.push_bind(trainer_id);
    }

    if let Some(trainer_name) = &params.trainer_name {
        query_builder.push(" AND t.name ILIKE ");
        query_builder.push_bind(format!("%{}%", trainer_name));
    }

    if let Some(main_parent_id) = params.main_parent_id {
        query_builder.push(" AND i.main_parent_id = ");
        query_builder.push_bind(main_parent_id);
    }

    // Filter by desired main character (p0 parent)
    if let Some(desired_main_chara_id) = params.desired_main_chara_id {
        query_builder.push(" AND i.main_chara_id = ");
        query_builder.push_bind(desired_main_chara_id);
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
        query_builder.push(" AND i.parent_rarity >= "); // Swapped per user request
        query_builder.push_bind(parent_rank);
    }

    if let Some(parent_rarity) = params.parent_rarity {
        query_builder.push(" AND i.parent_rank >= "); // Swapped per user request
        query_builder.push_bind(parent_rarity - 1);
    }

    // Add spark filters (multi-group AND logic)
    let blue_sparks_groups = process_spark_groups(&params.blue_sparks);
    add_multi_group_spark_conditions(&mut query_builder, "i.blue_sparks", &blue_sparks_groups);

    let pink_sparks_groups = process_spark_groups(&params.pink_sparks);
    add_multi_group_spark_conditions(&mut query_builder, "i.pink_sparks", &pink_sparks_groups);

    let green_sparks_groups = process_spark_groups(&params.green_sparks);
    add_multi_group_spark_conditions(&mut query_builder, "i.green_sparks", &green_sparks_groups);

    let white_sparks_groups = process_spark_groups(&params.white_sparks);
    add_multi_group_spark_conditions(&mut query_builder, "i.white_sparks", &white_sparks_groups);

    // Add 9-star spark filters (search across all stat types)
    if let Some(true) = params.blue_sparks_9star {
        add_9star_spark_conditions(&mut query_builder, "i.blue_sparks", 9);
    }

    if let Some(true) = params.pink_sparks_9star {
        add_9star_spark_conditions(&mut query_builder, "i.pink_sparks", 9);
    }

    if let Some(true) = params.green_sparks_9star {
        add_9star_spark_conditions(&mut query_builder, "i.green_sparks", 9);
    }

    // Add main parent spark filters
    let main_parent_blue_groups = process_spark_groups(&params.main_parent_blue_sparks);
    for group in main_parent_blue_groups {
        add_main_parent_spark_conditions(&mut query_builder, "i.main_blue_factors", &group);
    }

    let main_parent_pink_groups = process_spark_groups(&params.main_parent_pink_sparks);
    for group in main_parent_pink_groups {
        add_main_parent_spark_conditions(&mut query_builder, "i.main_pink_factors", &group);
    }

    let main_parent_green_groups = process_spark_groups(&params.main_parent_green_sparks);
    for group in main_parent_green_groups {
        add_main_parent_spark_conditions(&mut query_builder, "i.main_green_factors", &group);
    }

    let main_parent_white_groups = process_spark_groups(&params.main_parent_white_sparks);
    for group in main_parent_white_groups {
        add_spark_range_conditions(&mut query_builder, "i.main_white_factors", &group);
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

    let main_white_factors_groups = process_spark_groups(&params.main_white_factors);
    for group in main_white_factors_groups {
        add_spark_range_conditions(&mut query_builder, "i.main_white_factors", &group);
    }

    if let Some(min_main_white_count) = params.min_main_white_count {
        query_builder.push(" AND i.main_white_count >= ");
        query_builder.push_bind(min_main_white_count);
    }

    // GIN-optimized filter: include rows that have at least one matching optional spark
    // Filter each column separately based on what the user actually requested
    let white_sparks_expanded: Vec<i32> = optional_white_sparks_ids.iter()
        .flat_map(|&factor_id| (1..=9).map(move |level| factor_id * 10 + level))
        .collect();
    let main_white_factors_expanded: Vec<i32> = optional_main_white_factors_ids.iter()
        .flat_map(|&factor_id| (1..=9).map(move |level| factor_id * 10 + level))
        .collect();

    let has_white_sparks_filter = !white_sparks_expanded.is_empty();
    let has_main_white_factors_filter = !main_white_factors_expanded.is_empty();

    if has_white_sparks_filter && has_main_white_factors_filter {
        // Both specified: must match at least one in EITHER column (combined filter)
        query_builder.push(" AND (i.white_sparks && ARRAY[");
        for (i, val) in white_sparks_expanded.iter().enumerate() {
            if i > 0 { query_builder.push(","); }
            query_builder.push_bind(*val);
        }
        query_builder.push("]::int[] OR i.main_white_factors && ARRAY[");
        for (i, val) in main_white_factors_expanded.iter().enumerate() {
            if i > 0 { query_builder.push(","); }
            query_builder.push_bind(*val);
        }
        query_builder.push("]::int[])");
    } else if has_white_sparks_filter {
        // Only white_sparks specified: filter only on white_sparks
        query_builder.push(" AND i.white_sparks && ARRAY[");
        for (i, val) in white_sparks_expanded.iter().enumerate() {
            if i > 0 { query_builder.push(","); }
            query_builder.push_bind(*val);
        }
        query_builder.push("]::int[]");
    } else if has_main_white_factors_filter {
        // Only main_white_factors specified: filter only on main_white_factors
        query_builder.push(" AND i.main_white_factors && ARRAY[");
        for (i, val) in main_white_factors_expanded.iter().enumerate() {
            if i > 0 { query_builder.push(","); }
            query_builder.push_bind(*val);
        }
        query_builder.push("]::int[]");
    }

    if let Some(max_follower_num) = params.max_follower_num {
        query_builder.push(" AND (t.follower_num IS NULL OR t.follower_num <= ");
        query_builder.push_bind(max_follower_num);
        query_builder.push(")");
    }

    // OPTIMIZATION: Add EXISTS clause for support card filtering to force index usage
    if let Some(support_card_id) = params.support_card_id {
        query_builder.push(" AND EXISTS (SELECT 1 FROM support_card sc_exists WHERE sc_exists.account_id = t.account_id AND sc_exists.support_card_id = ");
        query_builder.push_bind(support_card_id);
        
        if let Some(min_lb) = params.min_limit_break {
             query_builder.push(" AND sc_exists.limit_break_count >= ");
             query_builder.push_bind(min_lb);
        }
        
        query_builder.push(")");
    }

    // Simplified ordering - use indexed columns
    // When optional scoring is provided, make it the PRIMARY sort criteria
    let has_optional_scoring = !optional_white_sparks_ids.is_empty() || !optional_main_white_factors_ids.is_empty();

    let order_by_clause = match params.sort_by.as_deref() {
        Some("affinity") | Some("affinity_score") => {
            // Affinity-based sorting - uses expression index
            // Use desired_main_chara_id for affinity if provided
            let affinity_player_id = params.desired_main_chara_id.or(params.player_chara_id);
            let affinity_expr = get_affinity_expression(affinity_player_id);
            if has_optional_scoring {
                // Optional scoring takes priority, then affinity as tiebreaker
                format!(" ORDER BY white_sparks_score DESC, main_white_factors_score DESC, {} DESC", affinity_expr)
            } else {
                format!(" ORDER BY {} DESC", affinity_expr)
            }
        }
        Some("win_count") => {
            if has_optional_scoring {
                " ORDER BY (white_sparks_score + main_white_factors_score) DESC, i.win_count DESC, t.account_id ASC".to_string()
            } else {
                " ORDER BY i.win_count DESC, t.account_id ASC".to_string()
            }
        }
        Some("white_count") => {
            if has_optional_scoring {
                " ORDER BY (white_sparks_score + main_white_factors_score) DESC, i.white_count DESC, t.account_id ASC".to_string()
            } else {
                " ORDER BY i.white_count DESC, t.account_id ASC".to_string()
            }
        }
        Some("parent_rank") => {
            if has_optional_scoring {
                " ORDER BY (white_sparks_score + main_white_factors_score) DESC, i.parent_rank DESC, t.account_id ASC".to_string()
            } else {
                " ORDER BY i.parent_rank DESC, t.account_id ASC".to_string()
            }
        }
        Some("submitted_at") | Some("last_updated") => {
            if has_optional_scoring {
                " ORDER BY (white_sparks_score + main_white_factors_score) DESC, t.last_updated DESC, t.account_id ASC".to_string()
            } else {
                " ORDER BY t.last_updated DESC, t.account_id ASC".to_string()
            }
        }
        Some("main_blue_factors") => {
            if has_optional_scoring {
                " ORDER BY (white_sparks_score + main_white_factors_score) DESC, i.main_blue_factors DESC, t.account_id ASC".to_string()
            } else {
                " ORDER BY i.main_blue_factors DESC, t.account_id ASC".to_string()
            }
        }
        Some("main_pink_factors") => {
            if has_optional_scoring {
                " ORDER BY (white_sparks_score + main_white_factors_score) DESC, i.main_pink_factors DESC, t.account_id ASC".to_string()
            } else {
                " ORDER BY i.main_pink_factors DESC, t.account_id ASC".to_string()
            }
        }
        Some("main_green_factors") => {
            if has_optional_scoring {
                " ORDER BY (white_sparks_score + main_white_factors_score) DESC, i.main_green_factors DESC, t.account_id ASC".to_string()
            } else {
                " ORDER BY i.main_green_factors DESC, t.account_id ASC".to_string()
            }
        }
        Some("main_white_count") => {
            if has_optional_scoring {
                " ORDER BY (white_sparks_score + main_white_factors_score) DESC, i.main_white_count DESC, t.account_id ASC".to_string()
            } else {
                " ORDER BY i.main_white_count DESC, t.account_id ASC".to_string()
            }
        }
        Some("experience") => {
            if has_optional_scoring {
                " ORDER BY (white_sparks_score + main_white_factors_score) DESC, sc.experience DESC NULLS LAST, t.account_id ASC".to_string()
            } else {
                " ORDER BY sc.experience DESC NULLS LAST, t.account_id ASC".to_string()
            }
        }
        Some("limit_break_count") => {
            if has_optional_scoring {
                " ORDER BY (white_sparks_score + main_white_factors_score) DESC, sc.limit_break_count DESC NULLS LAST, t.account_id ASC".to_string()
            } else {
                " ORDER BY sc.limit_break_count DESC NULLS LAST, t.account_id ASC".to_string()
            }
        }
        Some("follower_num") => {
            if has_optional_scoring {
                " ORDER BY (white_sparks_score + main_white_factors_score) DESC, COALESCE(t.follower_num, 999999) ASC, t.account_id ASC".to_string()
            } else {
                " ORDER BY COALESCE(t.follower_num, 999999) ASC, t.account_id ASC".to_string()
            }
        }
        Some("white_sparks_score") => {
            // Sort primarily by combined optional sparks score
            " ORDER BY (white_sparks_score + main_white_factors_score) DESC, t.account_id ASC".to_string()
        }
        Some("main_white_factors_score") => {
            // Sort primarily by combined optional sparks score
            " ORDER BY (white_sparks_score + main_white_factors_score) DESC, t.account_id ASC".to_string()
        }
        _ => {
            // Default: use affinity ordering for best results
            // Use desired_main_chara_id for affinity if provided
            let affinity_player_id = params.desired_main_chara_id.or(params.player_chara_id);
            let affinity_expr = get_affinity_expression(affinity_player_id);
            if has_optional_scoring {
                format!(" ORDER BY (white_sparks_score + main_white_factors_score) DESC, {} DESC", affinity_expr)
            } else {
                format!(" ORDER BY {} DESC", affinity_expr)
            }
        }
    };

    query_builder.push(&order_by_clause);
    query_builder.push(" LIMIT ");
    query_builder.push_bind(limit);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let query = query_builder.build();

    // Log the actual SQL query being executed
    // let sql = query.sql();
    // eprintln!(
    //     "📝 EXECUTING SQL (first 500 chars): {}",
    //     &sql.chars().take(500).collect::<String>()
    // );
    // eprintln!(
    //     "🔢 Query params: limit={}, offset={}, player_chara_id={:?}",
    //     limit, offset, params.player_chara_id
    // );
    // tracing::info!("📝 EXECUTING SQL: {}", sql);
    // tracing::info!(
    //     "🔢 Query params: limit={}, offset={}, player_chara_id={:?}",
    //     limit,
    //     offset,
    //     params.player_chara_id
    // );

    let query_start = std::time::Instant::now();
    let rows = query.fetch_all(&state.db).await?;
    let _query_duration = query_start.elapsed();
    // eprintln!(
    //     "⏱️  SQL EXECUTION TIME: {}ms (returned {} rows)",
    //     query_duration.as_millis(),
    //     rows.len()
    // );
    // tracing::info!(
    //     "⏱️  SQL EXECUTION TIME: {}ms (returned {} rows)",
    //     query_duration.as_millis(),
    //     rows.len()
    // );

    let mut records = Vec::new();
    for row in rows {
        let account_id: String = row.get("account_id");

        // Build support card directly from row (no JSON parsing needed)
        let support_card: Option<SupportCard> =
            if row.try_get::<Option<i32>, _>("support_card_id")?.is_some() {
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
        let inheritance: Option<Inheritance> =
            if row.try_get::<Option<i32>, _>("inheritance_id")?.is_some() {
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
                    affinity_score: row.try_get("affinity_score").ok(),
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

async fn execute_count_query(state: &AppState, params: &UnifiedSearchParams) -> Result<i64> {
    // For blank queries with no filters, use approximate count from stats table
    let is_blank_query = params.trainer_id.is_none()
        && params.trainer_name.is_none()
        && params.main_parent_id.is_none()
        && params.parent_left_id.is_none()
        && params.parent_right_id.is_none()
        && (params.parent_rank.is_none() || params.parent_rank == Some(1))
        && params.parent_rarity.is_none()
        && params.blue_sparks.is_empty()
        && params.pink_sparks.is_empty()
        && params.green_sparks.is_empty()
        && params.white_sparks.is_empty()
        && params.blue_sparks_9star.is_none()
        && params.pink_sparks_9star.is_none()
        && params.green_sparks_9star.is_none()
        && params.main_parent_blue_sparks.is_empty()
        && params.main_parent_pink_sparks.is_empty()
        && params.main_parent_green_sparks.is_empty()
        && params.main_parent_white_sparks.is_empty()
        && params.support_card_id.is_none()
        && params.min_limit_break.is_none()
        && params.max_limit_break.is_none()
        && params.min_experience.is_none()
        && (params.min_win_count.is_none() || params.min_win_count == Some(0))
        && (params.min_white_count.is_none() || params.min_white_count == Some(0))
        && params.min_main_blue_factors.is_none()
        && params.min_main_pink_factors.is_none()
        && params.min_main_green_factors.is_none()
        && params.main_white_factors.is_empty()
        && params.optional_white_sparks.is_empty()
        && params.optional_main_white_factors.is_empty()
        && (params.min_main_white_count.is_none() || params.min_main_white_count == Some(0))
        && params.desired_main_chara_id.is_none()
        && params.player_chara_id.is_none()
        && (params.max_follower_num.is_none() || params.max_follower_num == Some(1000) || params.max_follower_num == Some(999));

    if is_blank_query {
        tracing::info!("📊 COUNT: Using stats_counts table (instant)");
        // Use materialized view for instant count (no actual counting!)
        let count: i64 =
            sqlx::query_scalar("SELECT COALESCE(trainer_count, 0) FROM stats_counts LIMIT 1")
                .fetch_one(&state.db)
                .await?;

        return Ok(count);
    }

    // Cache counts for common filter combinations (they change infrequently)
    // Build comprehensive cache key based on ALL filters to avoid returning wrong counts
    let cache_key = format!(
        "count:type={}:sc_id={}:lb_min={}:lb_max={}:exp_min={}:main_parent={}:p_left={}:p_right={}:p_rank={}:p_rarity={}:blue={}:pink={}:green={}:white={}:blue9={}:pink9={}:green9={}:mp_blue={}:mp_pink={}:mp_green={}:mp_white={}:win={}:wh_cnt={}:trainer={}:trainer_name={}:desired_main={}",
        params.search_type.as_deref().unwrap_or("all"),
        params.support_card_id.map(|v| v.to_string()).unwrap_or_else(|| "any".to_string()),
        params.min_limit_break.map(|v| v.to_string()).unwrap_or_else(|| "any".to_string()),
        params.max_limit_break.map(|v| v.to_string()).unwrap_or_else(|| "any".to_string()),
        params.min_experience.map(|v| v.to_string()).unwrap_or_else(|| "any".to_string()),
        params.main_parent_id.map(|v| v.to_string()).unwrap_or_else(|| "any".to_string()),
        params.parent_left_id.map(|v| v.to_string()).unwrap_or_else(|| "any".to_string()),
        params.parent_right_id.map(|v| v.to_string()).unwrap_or_else(|| "any".to_string()),
        params.parent_rank.map(|v| v.to_string()).unwrap_or_else(|| "any".to_string()),
        params.parent_rarity.map(|v| v.to_string()).unwrap_or_else(|| "any".to_string()),
        if params.blue_sparks.is_empty() { "any".to_string() } else { format!("{:?}", params.blue_sparks) },
        if params.pink_sparks.is_empty() { "any".to_string() } else { format!("{:?}", params.pink_sparks) },
        if params.green_sparks.is_empty() { "any".to_string() } else { format!("{:?}", params.green_sparks) },
        if params.white_sparks.is_empty() { "any".to_string() } else { format!("{:?}", params.white_sparks) },
        params.blue_sparks_9star.map(|v| v.to_string()).unwrap_or_else(|| "any".to_string()),
        params.pink_sparks_9star.map(|v| v.to_string()).unwrap_or_else(|| "any".to_string()),
        params.green_sparks_9star.map(|v| v.to_string()).unwrap_or_else(|| "any".to_string()),
        if params.main_parent_blue_sparks.is_empty() { "any".to_string() } else { format!("{:?}", params.main_parent_blue_sparks) },
        if params.main_parent_pink_sparks.is_empty() { "any".to_string() } else { format!("{:?}", params.main_parent_pink_sparks) },
        if params.main_parent_green_sparks.is_empty() { "any".to_string() } else { format!("{:?}", params.main_parent_green_sparks) },
        if params.main_parent_white_sparks.is_empty() { "any".to_string() } else { format!("{:?}", params.main_parent_white_sparks) },
        params.min_win_count.map(|v| v.to_string()).unwrap_or_else(|| "any".to_string()),
        params.min_white_count.map(|v| v.to_string()).unwrap_or_else(|| "any".to_string()),
        params.trainer_id.as_ref().unwrap_or(&"any".to_string()),
        params.trainer_name.as_ref().unwrap_or(&"any".to_string()),
        params.desired_main_chara_id.map(|v| v.to_string()).unwrap_or_else(|| "any".to_string())
    );

    // Try to get cached count (cache for 5 minutes)
    if let Some(cached_count) = crate::cache::get::<i64>(&cache_key) {
        tracing::info!("🎯 CACHE HIT: count - {}", cached_count);
        return Ok(cached_count);
    }
    tracing::info!("❌ CACHE MISS: count query");

    // Unified count query: always start from inheritance
    // OPTIMIZATION: Wrap in subquery with LIMIT to prevent slow full table scans
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
        SELECT COUNT(*) FROM (
            SELECT 1
            FROM inheritance i
            INNER JOIN trainer t ON i.account_id = t.account_id
            WHERE (t.follower_num IS NULL OR t.follower_num < 1000)
    "#,
    );

    // Player exclusion - don't show inheritances where player is the main character
    // Use the same player ID as affinity calculation (desired_main_chara_id takes precedence)
    let affinity_player_id = params.desired_main_chara_id.or(params.player_chara_id);
    if let Some(player_id) = affinity_player_id {
        // Only exclude if we're NOT filtering for this specific character as main parent
        // (when desired_main_chara_id is set, we WANT that character as main parent)
        if params.desired_main_chara_id.is_none() {
            query_builder.push(" AND i.main_chara_id != ");
            query_builder.push_bind(player_id);
        }
    }

    // Apply inheritance filters directly (no EXISTS needed)
    if let Some(trainer_id) = &params.trainer_id {
        query_builder.push(" AND t.account_id = ");
        query_builder.push_bind(trainer_id);
    }

    // OPTIMIZATION: Use EXISTS for support card filtering
    if params.support_card_id.is_some() 
        || params.min_limit_break.is_some() 
        || params.max_limit_break.is_some() 
        || params.min_experience.is_some() 
    {
        query_builder.push(" AND EXISTS (SELECT 1 FROM support_card sc_ex WHERE sc_ex.account_id = i.account_id");
        
        if let Some(support_card_id) = params.support_card_id {
            query_builder.push(" AND sc_ex.support_card_id = ");
            query_builder.push_bind(support_card_id);
        }

        if let Some(min_limit_break) = params.min_limit_break {
            query_builder.push(" AND sc_ex.limit_break_count >= ");
            query_builder.push_bind(min_limit_break);
        }

        if let Some(max_limit_break) = params.max_limit_break {
            query_builder.push(" AND sc_ex.limit_break_count <= ");
            query_builder.push_bind(max_limit_break);
        }

        if let Some(min_experience) = params.min_experience {
            query_builder.push(" AND sc_ex.experience >= ");
            query_builder.push_bind(min_experience);
        }
        
        query_builder.push(")");
    }

    // Player exclusion - use the same logic as search query
    let affinity_player_id = params.desired_main_chara_id.or(params.player_chara_id);
    if let Some(player_id) = affinity_player_id {
        // Only exclude if we're NOT filtering for this specific character as main parent
        if params.desired_main_chara_id.is_none() {
            query_builder.push(" AND i.main_chara_id != ");
            query_builder.push_bind(player_id);
        }
    }

    // Apply inheritance filters (only if inheritance table is joined)
    if let Some(trainer_id) = &params.trainer_id {
        query_builder.push(" AND t.account_id = ");
        query_builder.push_bind(trainer_id);
    }

    if let Some(trainer_name) = &params.trainer_name {
        query_builder.push(" AND t.name ILIKE ");
        query_builder.push_bind(format!("%{}%", trainer_name));
    }

    if let Some(main_parent_id) = params.main_parent_id {
        query_builder.push(" AND i.main_parent_id = ");
        query_builder.push_bind(main_parent_id);
    }

    // Filter by desired main character (p0 parent)
    if let Some(desired_main_chara_id) = params.desired_main_chara_id {
        query_builder.push(" AND i.main_chara_id = ");
        query_builder.push_bind(desired_main_chara_id);
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
        query_builder.push(" AND i.parent_rarity >= "); // Swapped per user request
        query_builder.push_bind(parent_rank);
    }

    if let Some(parent_rarity) = params.parent_rarity {
        query_builder.push(" AND i.parent_rank >= "); // Swapped per user request
        query_builder.push_bind(parent_rarity - 1);
    }

    // Add spark filters (multi-group AND logic)
    let blue_sparks_groups = process_spark_groups(&params.blue_sparks);
    add_multi_group_spark_conditions(&mut query_builder, "i.blue_sparks", &blue_sparks_groups);

    let pink_sparks_groups = process_spark_groups(&params.pink_sparks);
    add_multi_group_spark_conditions(&mut query_builder, "i.pink_sparks", &pink_sparks_groups);

    let green_sparks_groups = process_spark_groups(&params.green_sparks);
    add_multi_group_spark_conditions(&mut query_builder, "i.green_sparks", &green_sparks_groups);

    let white_sparks_groups = process_spark_groups(&params.white_sparks);
    add_multi_group_spark_conditions(&mut query_builder, "i.white_sparks", &white_sparks_groups);

    // Add 9-star spark filters (search across all stat types)
    if let Some(true) = params.blue_sparks_9star {
        add_9star_spark_conditions(&mut query_builder, "i.blue_sparks", 9);
    }

    if let Some(true) = params.pink_sparks_9star {
        add_9star_spark_conditions(&mut query_builder, "i.pink_sparks", 9);
    }

    if let Some(true) = params.green_sparks_9star {
        add_9star_spark_conditions(&mut query_builder, "i.green_sparks", 9);
    }

    // Add main parent spark filters
    let main_parent_blue_groups = process_spark_groups(&params.main_parent_blue_sparks);
    for group in main_parent_blue_groups {
        add_main_parent_spark_conditions(&mut query_builder, "i.main_blue_factors", &group);
    }

    let main_parent_pink_groups = process_spark_groups(&params.main_parent_pink_sparks);
    for group in main_parent_pink_groups {
        add_main_parent_spark_conditions(&mut query_builder, "i.main_pink_factors", &group);
    }

    let main_parent_green_groups = process_spark_groups(&params.main_parent_green_sparks);
    for group in main_parent_green_groups {
        add_main_parent_spark_conditions(&mut query_builder, "i.main_green_factors", &group);
    }

    let main_parent_white_groups = process_spark_groups(&params.main_parent_white_sparks);
    for group in main_parent_white_groups {
        add_spark_range_conditions(&mut query_builder, "i.main_white_factors", &group);
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

    let main_white_factors_groups = process_spark_groups(&params.main_white_factors);
    for group in main_white_factors_groups {
        add_spark_range_conditions(&mut query_builder, "i.main_white_factors", &group);
    }

    if let Some(min_main_white_count) = params.min_main_white_count {
        query_builder.push(" AND i.main_white_count >= ");
        query_builder.push_bind(min_main_white_count);
    }

    if let Some(max_follower_num) = params.max_follower_num {
        query_builder.push(" AND (t.follower_num IS NULL OR t.follower_num <= ");
        query_builder.push_bind(max_follower_num);
        query_builder.push(")");
    }

    // Optimization: If filtering by support card ID, add an EXISTS clause to help the planner
    // NOTE: This is already handled above in the main count query logic, removing duplicate block
    /*
    if let Some(support_card_id) = params.support_card_id {
        query_builder.push(" AND EXISTS (SELECT 1 FROM support_card sc_ex WHERE sc_ex.account_id = i.account_id AND sc_ex.support_card_id = ");
        query_builder.push_bind(support_card_id);
        
        if let Some(min_lb) = params.min_limit_break {
             query_builder.push(" AND sc_ex.limit_break_count >= ");
             query_builder.push_bind(min_lb);
        }

        if let Some(max_lb) = params.max_limit_break {
             query_builder.push(" AND sc_ex.limit_break_count <= ");
             query_builder.push_bind(max_lb);
        }

        if let Some(min_exp) = params.min_experience {
             query_builder.push(" AND sc_ex.experience >= ");
             query_builder.push_bind(min_exp);
        }
        
        query_builder.push(")");
    }
    */

    // Cap the count at 10,001 to indicate if there are more results than the limit
    query_builder.push(" LIMIT 10001) AS sub");
    let query = query_builder.build();

    let query_start = std::time::Instant::now();
    let row = query.fetch_one(&state.db).await?;
    let count: i64 = row.get::<i64, _>(0);
    let query_duration = query_start.elapsed();
    tracing::info!(
        "⏱️  COUNT QUERY EXECUTED: {}ms (result={})",
        query_duration.as_millis(),
        count
    );

    // Cache the count for 5 minutes (counts don't change frequently)
    if crate::cache::set(&cache_key, &count, std::time::Duration::from_secs(300)).is_ok() {
        tracing::info!("💾 CACHE SET: count={}", count);
    }

    Ok(count)
}

pub async fn get_unified_count(State(state): State<AppState>) -> Result<Json<serde_json::Value>> {
    let total_inheritance_count = sqlx::query("SELECT COUNT(*) FROM inheritance")
        .fetch_one(&state.db)
        .await?
        .get::<i64, _>(0);

    let total_support_card_accounts =
        sqlx::query("SELECT COUNT(DISTINCT account_id) FROM support_card")
            .fetch_one(&state.db)
            .await?
            .get::<i64, _>(0);

    let available_inheritance_count = sqlx::query(
        r#"
        SELECT COUNT(*) 
        FROM inheritance i 
        INNER JOIN trainer t ON i.account_id = t.account_id
        WHERE (t.follower_num IS NULL OR t.follower_num < 1000)
        "#,
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
        "#,
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
