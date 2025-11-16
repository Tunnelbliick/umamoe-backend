use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::NaiveDateTime;

/// Circle model representing a game circle/guild
#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Circle {
    pub circle_id: i64,
    pub name: String,
    pub comment: Option<String>,
    pub leader_viewer_id: Option<i64>,
    pub leader_name: Option<String>,
    pub member_count: Option<i32>,
    pub join_style: Option<i32>,
    pub policy: Option<i32>,
    pub created_at: Option<NaiveDateTime>,
    pub last_updated: Option<NaiveDateTime>,
    pub monthly_rank: Option<i32>,
    pub monthly_point: Option<i64>,
    pub last_month_rank: Option<i32>,
    pub last_month_point: Option<i64>,
}

/// Circle member fans monthly tracking
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CircleMemberFansMonthly {
    pub id: i32,
    pub circle_id: i64,
    pub viewer_id: i64,
    pub trainer_name: Option<String>,
    pub year: i32,
    pub month: i32,
    pub daily_fans: Vec<i32>,
    pub last_updated: Option<NaiveDateTime>,
}

/// Response for circle with aggregated member fans data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CircleWithMembers {
    #[serde(flatten)]
    pub circle: Circle,
    pub members: Vec<CircleMemberFansMonthly>,
}

/// Query parameters for searching circles
#[derive(Debug, Deserialize, Clone)]
pub struct CircleSearchParams {
    pub circle_id: Option<i64>,
    pub viewer_id: Option<i64>,
    pub name: Option<String>,
    pub min_members: Option<i32>,
    pub max_members: Option<i32>,
    pub join_style: Option<i32>,
    pub policy: Option<i32>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Query parameters for circle member fans data
#[derive(Debug, Deserialize, Clone)]
pub struct CircleMemberFansParams {
    pub circle_id: Option<i64>,
    pub viewer_id: Option<i64>,
    pub year: Option<i32>,
    pub month: Option<i32>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Response for paginated circle results
#[derive(Debug, Serialize, Clone)]
pub struct CircleSearchResponse {
    pub circles: Vec<Circle>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Response for paginated circle member fans results
#[derive(Debug, Serialize, Clone)]
pub struct CircleMemberFansResponse {
    pub records: Vec<CircleMemberFansMonthly>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Circle statistics
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CircleStats {
    pub circle_id: i64,
    pub total_members: i32,
    pub total_monthly_fans: i64,
    pub avg_member_fans: Option<f64>,
    pub top_contributor_viewer_id: Option<i64>,
    pub top_contributor_fans: Option<i64>,
}

/// Request body for creating/updating a circle
#[derive(Debug, Deserialize, Clone)]
pub struct CreateCircleRequest {
    pub circle_id: i64,
    pub name: String,
    pub comment: Option<String>,
    pub leader_viewer_id: Option<i64>,
    pub member_count: Option<i32>,
    pub join_style: Option<i32>,
    pub policy: Option<i32>,
    pub monthly_rank: Option<i32>,
    pub monthly_point: Option<i64>,
    pub last_month_rank: Option<i32>,
    pub last_month_point: Option<i64>,
}

/// Request body for creating/updating circle member fans data
#[derive(Debug, Deserialize, Clone)]
pub struct CreateCircleMemberFansRequest {
    pub circle_id: i64,
    pub viewer_id: i64,
    pub year: i32,
    pub month: i32,
    pub daily_fans: Vec<i32>,
}
