use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Deserialize)]
pub struct DailyVisitRequest {
    pub date: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatsResponse {
    pub today: TodayStats,
    pub rolling_averages: RollingStats,
    pub daily_data: Vec<DailyStatsResponse>,
    pub totals: TotalStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TodayStats {
    pub total_visitors: i32,
    pub unique_visitors: i32,
    pub inheritance_uploads: i32,
    pub total_inheritance_records: i32,
    pub total_support_card_records: i32,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DailyStatsResponse {
    pub date: chrono::NaiveDate,
    pub total_visits: i64,
    pub unique_visitors: i64,
    pub inheritance_uploads: i64,
    pub support_card_uploads: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TotalStats {
    pub total_records: i64,
    pub inheritance_records: i64,
    pub support_card_records: i64,
    pub total_votes: i64,
    pub total_visitors: i64,
    pub total_accounts_tracked: i64,
    pub total_circles_tracked: i64,
    pub total_characters: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RollingStats {
    pub visitors_7_day: f64,
    pub visitors_30_day: f64,
    pub unique_visitors_7_day: f64,
    pub unique_visitors_30_day: f64,
    pub uploads_7_day: f64,
    pub uploads_30_day: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FriendlistReportResponse {
    pub success: bool,
    pub message: String,
}
