use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{
    errors::AppError,
    models::{Circle, CircleMemberFansMonthly},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct CircleQueryParams {
    /// Query by viewer ID - will find their circle
    pub viewer_id: Option<i64>,
    /// Query by circle ID directly
    pub circle_id: Option<i64>,
    /// Filter members by month (1-12)
    pub month: Option<i32>,
    /// Filter members by year
    pub year: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CircleListParams {
    /// Page number (0-indexed)
    #[serde(default)]
    pub page: Option<i64>,
    /// Results per page
    #[serde(default)]
    pub limit: Option<i64>,
    /// Search by circle name (partial match)
    pub name: Option<String>,
    /// Minimum member count
    pub min_members: Option<i32>,
    /// Minimum monthly rank (lower is better)
    pub max_rank: Option<i32>,
    /// Sort by field (name, member_count, monthly_rank, monthly_point)
    pub sort_by: Option<String>,
    /// Sort direction (asc, desc)
    pub sort_dir: Option<String>,
    /// General search query (circle ID/name, leader ID/name, member ID/name)
    pub query: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CircleResponse {
    pub circle: Circle,
    pub members: Vec<CircleMemberFansMonthly>,
}

#[derive(Debug, Serialize)]
pub struct CircleWithRank {
    #[serde(flatten)]
    pub circle: Circle,
}

#[derive(Debug, Serialize)]
pub struct CircleListResponse {
    pub circles: Vec<CircleWithRank>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
    pub total_pages: i64,
}

/// Create the circles router
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_circle))
        .route("/list", get(list_circles))
}

/// GET /api/circles - Get circle information and member fan counts
///
/// Parameters:
/// - viewer_id: Get circle for a specific viewer (will add to tasks if not found)
/// - circle_id: Get circle by ID directly
///
/// Returns circle info with all member fan count data
pub async fn get_circle(
    Query(params): Query<CircleQueryParams>,
    State(state): State<AppState>,
) -> Result<Json<CircleResponse>, AppError> {
    // Validate that at least one parameter is provided
    if params.viewer_id.is_none() && params.circle_id.is_none() {
        return Err(AppError::BadRequest(
            "Either viewer_id or circle_id must be provided".to_string(),
        ));
    }

    let circle = if let Some(viewer_id) = params.viewer_id {
        // Query by viewer_id - first check if viewer exists in circle_member_fans_monthly
        let member_record = sqlx::query!(
            r#"
            SELECT circle_id 
            FROM circle_member_fans_monthly 
            WHERE viewer_id = $1 
            LIMIT 1
            "#,
            viewer_id
        )
        .fetch_optional(&state.db)
        .await?;

        match member_record {
            Some(record) => {
                // Viewer found, get their circle
                let circle_id = record.circle_id;
                fetch_circle_by_id(&state.db, circle_id).await?
            }
            None => {
                // Viewer not found - add to tasks for later fetching
                add_viewer_to_tasks(&state.db, viewer_id).await?;

                return Err(AppError::NotFound(format!(
                    "Viewer {} not found in any circle. Added to task queue for fetching.",
                    viewer_id
                )));
            }
        }
    } else if let Some(circle_id) = params.circle_id {
        // Query by circle_id directly
        fetch_circle_by_id(&state.db, circle_id).await?
    } else {
        unreachable!("Already validated at least one param exists");
    };

    // Get all members and their fan counts for this circle
    let members = fetch_circle_members(&state.db, circle.circle_id, params.year, params.month).await?;

    Ok(Json(CircleResponse { circle, members }))
}

/// GET /api/circles/list - List all circles with pagination and filtering
///
/// Parameters:
/// - page: Page number (0-indexed, default: 0)
/// - limit: Results per page (default: 100, max: 100)
/// - name: Filter by circle name (partial match, case-insensitive)
/// - min_members: Minimum member count
/// - max_rank: Maximum monthly rank (lower is better, e.g., rank 1 is best)
/// - sort_by: Field to sort by (name, member_count, monthly_rank, monthly_point)
/// - sort_dir: Sort direction (asc, desc)
///
/// Returns paginated list of circles
pub async fn list_circles(
    Query(params): Query<CircleListParams>,
    State(state): State<AppState>,
) -> Result<Json<CircleListResponse>, AppError> {
    let page = params.page.unwrap_or(0);
    let limit = params.limit.unwrap_or(100).min(100);
    let offset = page * limit;

    // Only calculate live ranks if we are NOT searching (or if explicitly requested)
    // For search queries, we can rely on stored monthly_rank to avoid expensive window functions
    let use_live_ranks = params.query.is_none();

    let mut with_parts = Vec::new();

    if use_live_ranks {
        with_parts.push(r#"
            GlobalRanks AS (
                SELECT 
                    circle_id, 
                    RANK() OVER (ORDER BY monthly_point DESC NULLS LAST) as live_rank,
                    RANK() OVER (ORDER BY yesterday_points DESC NULLS LAST) as live_yesterday_rank
                FROM circles
                WHERE (archived IS NULL OR archived = false)
                  AND last_updated >= date_trunc('month', CURRENT_DATE)
                  AND last_updated < date_trunc('month', CURRENT_DATE) + interval '1 month'
            )
        "#.trim().to_string());
    }

    // If search query is present, add MatchingCircles CTE to optimize search
    let mut join_matching_circles = String::new();
    
    if let Some(query) = &params.query {
        // Skip very short queries that would match too many results
        let query_trimmed = query.trim();
        if query_trimmed.len() >= 2 {
            let search_pattern = format!("%{}%", query_trimmed.replace("'", "''"));
            let search_exact = query_trimmed.replace("'", "''");
            let is_number = query_trimmed.parse::<i64>().is_ok();

        let mut union_parts = Vec::new();

        // 1. Search by Circle Name
        union_parts.push(format!("SELECT circle_id FROM circles WHERE name ILIKE '{}'", search_pattern));

        // 2. Search by Leader Name
        union_parts.push(format!(
            "SELECT c.circle_id FROM circles c JOIN trainer t ON c.leader_viewer_id::text = t.account_id WHERE t.name ILIKE '{}'", 
            search_pattern
        ));

        // 3. Search by Member Name
        union_parts.push(format!(
            r#"
            SELECT cm.circle_id 
            FROM circle_member_fans_monthly cm 
            JOIN trainer tm ON cm.viewer_id::text = tm.account_id 
            WHERE cm.year = extract(year from current_date)::int 
              AND cm.month = extract(month from current_date)::int 
              AND tm.name ILIKE '{}'
            "#,
            search_pattern
        ));

        if is_number {
            // 4. Search by Circle ID
            union_parts.push(format!("SELECT circle_id FROM circles WHERE circle_id = {}", search_exact));
            
            // 5. Search by Leader ID
            union_parts.push(format!("SELECT circle_id FROM circles WHERE leader_viewer_id = {}", search_exact));
            
            // 6. Search by Member ID
            union_parts.push(format!(
                r#"
                SELECT circle_id 
                FROM circle_member_fans_monthly 
                WHERE viewer_id = {} 
                  AND year = extract(year from current_date)::int 
                  AND month = extract(month from current_date)::int
                "#,
                search_exact
            ));
        }

        with_parts.push(format!("MatchingCircles AS ({})", union_parts.join(" UNION ")));

        join_matching_circles = "INNER JOIN MatchingCircles mc ON c.circle_id = mc.circle_id".to_string();
        } // end of query_trimmed.len() >= 2 check
    }

    let with_clause = if with_parts.is_empty() {
        String::new()
    } else {
        format!("WITH {}", with_parts.join(", "))
    };

    let join_global_ranks = if use_live_ranks {
        "LEFT JOIN GlobalRanks gr ON c.circle_id = gr.circle_id"
    } else {
        ""
    };

    let rank_column = if use_live_ranks {
        "COALESCE(gr.live_rank::integer, c.monthly_rank)"
    } else {
        "c.monthly_rank"
    };

    let yesterday_rank_column = if use_live_ranks {
        "COALESCE(gr.live_yesterday_rank::integer, c.yesterday_rank)"
    } else {
        "c.yesterday_rank"
    };

    // Build dynamic query
    let mut count_query = format!(
        "{} SELECT COUNT(*) FROM circles c {} LEFT JOIN trainer t ON c.leader_viewer_id::text = t.account_id {} WHERE 1=1", 
        with_clause, 
        join_global_ranks,
        join_matching_circles
    );
    
    let mut select_query = format!(
        r#"
        {}
        SELECT 
            c.circle_id,
            c.name,
            c.comment,
            c.leader_viewer_id,
            t.name as leader_name,
            c.member_count,
            c.join_style,
            c.policy,
            c.created_at,
            c.last_updated,
            {} as monthly_rank,
            c.monthly_point,
            c.last_month_rank,
            c.last_month_point,
            c.archived,
            c.yesterday_updated,
            c.yesterday_points,
            {} as yesterday_rank
        FROM circles c
        {}
        LEFT JOIN trainer t ON c.leader_viewer_id::text = t.account_id
        {}
        WHERE 1=1
        "#,
        with_clause,
        rank_column,
        yesterday_rank_column,
        join_global_ranks,
        join_matching_circles
    );

    let mut conditions = Vec::new();

    // Only show circles updated this month to ensure points are current
    conditions.push("c.last_updated >= date_trunc('month', CURRENT_DATE)".to_string());
    conditions.push("c.last_updated < date_trunc('month', CURRENT_DATE) + interval '1 month'".to_string());
    // Exclude archived circles
    conditions.push("(c.archived IS NULL OR c.archived = false)".to_string());

    // Name filter
    if let Some(name) = &params.name {
        conditions.push(format!("c.name ILIKE '%{}%'", name.replace("'", "''")));
    }

    // General Search Query - handled by CTE now, no extra conditions needed here
    // But we keep the parameter check to avoid unused variable warning if we removed it completely
    // (Actually we used it above to build CTE)

    // Min members filter
    if let Some(min_members) = params.min_members {
        conditions.push(format!("c.member_count >= {}", min_members));
    }

    // Max rank filter (lower rank number is better)
    if let Some(max_rank) = params.max_rank {
        // Use the calculated rank for filtering
        conditions.push(format!("COALESCE(gr.live_rank, c.monthly_rank) <= {}", max_rank));
    }

    // Add conditions to queries
    for condition in &conditions {
        count_query.push_str(&format!(" AND {}", condition));
        select_query.push_str(&format!(" AND {}", condition));
    }

    // Get total count
    let total: i64 = sqlx::query_scalar(&count_query)
        .fetch_one(&state.db)
        .await?;

    // Add sorting
    let sort_by = params.sort_by.as_deref().unwrap_or("rank");
    let sort_dir = params.sort_dir.as_deref().unwrap_or("asc");

    let order_clause = match sort_by {
        "name" => format!(" ORDER BY c.name {}, c.circle_id ASC", sort_dir.to_uppercase()),
        "member_count" => format!(
            " ORDER BY c.member_count {} NULLS LAST, c.circle_id ASC",
            sort_dir.to_uppercase()
        ),
        "rank" | "monthly_rank" => format!(
            " ORDER BY monthly_rank {} NULLS LAST, c.circle_id ASC",
            sort_dir.to_uppercase()
        ),
        "monthly_point" => format!(
            " ORDER BY c.monthly_point {} NULLS LAST, c.circle_id ASC",
            sort_dir.to_uppercase()
        ),
        _ => " ORDER BY monthly_rank ASC NULLS LAST, c.circle_id ASC".to_string(),
    };

    select_query.push_str(&order_clause);
    select_query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

    // Execute query
    let circles = sqlx::query_as::<_, Circle>(&select_query)
        .fetch_all(&state.db)
        .await?;

    let circles_with_rank: Vec<CircleWithRank> = circles
        .into_iter()
        .map(|circle| CircleWithRank { circle })
        .collect();

    let total_pages = if limit > 0 {
        ((total as f64) / (limit as f64)).ceil() as i64
    } else {
        0
    };

    Ok(Json(CircleListResponse {
        circles: circles_with_rank,
        total,
        page,
        limit,
        total_pages,
    }))
}

/// Fetch circle by ID
async fn fetch_circle_by_id(pool: &PgPool, circle_id: i64) -> Result<Circle, AppError> {
    let circle = sqlx::query_as::<_, Circle>(
        r#"
        SELECT 
            c.circle_id,
            c.name,
            c.comment,
            c.leader_viewer_id,
            t.name as leader_name,
            c.member_count,
            c.join_style,
            c.policy,
            c.created_at,
            c.last_updated,
            c.monthly_rank,
            c.monthly_point,
            c.last_month_rank,
            c.last_month_point,
            c.archived,
            c.yesterday_updated,
            c.yesterday_points,
            c.yesterday_rank
        FROM circles c
        LEFT JOIN trainer t ON c.leader_viewer_id::text = t.account_id
        WHERE c.circle_id = $1
        "#,
    )
    .bind(circle_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Circle {} not found", circle_id)))?;

    Ok(circle)
}

/// Fetch all members and their fan counts for a circle
async fn fetch_circle_members(
    pool: &PgPool,
    circle_id: i64,
    year: Option<i32>,
    month: Option<i32>,
) -> Result<Vec<CircleMemberFansMonthly>, AppError> {
    use chrono::Datelike;
    
    // Default to current date if not provided
    let (target_year, target_month) = if year.is_none() || month.is_none() {
        let now = chrono::Local::now();
        (
            year.unwrap_or(now.year()),
            month.unwrap_or(now.month() as i32)
        )
    } else {
        (year.unwrap(), month.unwrap())
    };

    // PostgreSQL returns integer arrays as Vec<i32>, but query_as! infers Vec<i64>
    // We need to handle the conversion manually
    let records = sqlx::query!(
        r#"
        SELECT 
            cm.id,
            cm.circle_id,
            cm.viewer_id,
            t.name as "trainer_name?",
            cm.year,
            cm.month,
            cm.daily_fans,
            cm.last_updated
        FROM circle_member_fans_monthly cm
        LEFT JOIN trainer t ON cm.viewer_id::text = t.account_id
        WHERE cm.circle_id = $1 AND cm.year = $2 AND cm.month = $3
        ORDER BY cm.viewer_id
        "#,
        circle_id,
        target_year,
        target_month
    )
    .fetch_all(pool)
    .await?;

    let members = records
        .into_iter()
        .map(|rec| CircleMemberFansMonthly {
            id: rec.id,
            circle_id: rec.circle_id,
            viewer_id: rec.viewer_id,
            trainer_name: rec.trainer_name,
            year: rec.year,
            month: rec.month,
            daily_fans: rec.daily_fans.into_iter().map(|v| v as i32).collect(),
            last_updated: rec.last_updated,
        })
        .collect();

    Ok(members)
}

/// Add a viewer to the tasks queue for later fetching
async fn add_viewer_to_tasks(pool: &PgPool, viewer_id: i64) -> Result<(), AppError> {
    // Insert into tasks table with viewer_id in task_data
    // account_id is for the worker that processes the task, so we leave it NULL
    sqlx::query!(
        r#"
        INSERT INTO tasks (task_type, task_data, status, created_at, updated_at)
        VALUES ('fetch_circle', $1, 'pending', NOW(), NOW())
        ON CONFLICT DO NOTHING
        "#,
        serde_json::json!({ "viewer_id": viewer_id })
    )
    .execute(pool)
    .await?;

    Ok(())
}
