use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

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
    pub archived: Option<bool>,
    pub yesterday_updated: Option<NaiveDateTime>,
    pub yesterday_points: Option<i64>,
    pub yesterday_rank: Option<i32>,
}

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
