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
}

#[derive(Debug, Serialize)]
pub struct CircleResponse {
    pub circle: Circle,
    pub members: Vec<CircleMemberFansMonthly>,
}

/// Create the circles router
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_circle))
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
    let members = fetch_circle_members(&state.db, circle.circle_id).await?;

    Ok(Json(CircleResponse { circle, members }))
}

/// Fetch circle by ID
async fn fetch_circle_by_id(pool: &PgPool, circle_id: i64) -> Result<Circle, AppError> {
    let circle = sqlx::query_as!(
        Circle,
        r#"
        SELECT 
            c.circle_id,
            c.name,
            c.comment,
            c.leader_viewer_id,
            t.name as "leader_name?",
            c.member_count,
            c.join_style,
            c.policy,
            c.created_at,
            c.last_updated,
            c.monthly_rank,
            c.monthly_point,
            c.last_month_rank,
            c.last_month_point
        FROM circles c
        LEFT JOIN trainer t ON c.leader_viewer_id::text = t.account_id
        WHERE c.circle_id = $1
        "#,
        circle_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Circle {} not found", circle_id)))?;

    Ok(circle)
}

/// Fetch all members and their fan counts for a circle
async fn fetch_circle_members(
    pool: &PgPool,
    circle_id: i64,
) -> Result<Vec<CircleMemberFansMonthly>, AppError> {
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
        WHERE cm.circle_id = $1
        ORDER BY cm.year DESC, cm.month DESC, cm.viewer_id
        "#,
        circle_id
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
