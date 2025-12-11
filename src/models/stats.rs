use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// Visitor tracking models
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct VisitorTracking {
    pub id: String,
    pub visitor_id: String,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub visited_at: DateTime<Utc>,
    pub page_url: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DailyStats {
    pub date: chrono::NaiveDate,
    pub total_visits: i64,
    pub unique_visitors: i64,
    pub records_submitted: i64,
    pub total_votes: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SiteStats {
    pub daily_stats: Vec<DailyStats>,
    pub rolling_averages: RollingAverages,
    pub total_records: i64,
    pub total_votes: i64,
    pub total_visitors: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RollingAverages {
    pub visits_7_day: f64,
    pub visits_30_day: f64,
    pub unique_visitors_7_day: f64,
    pub unique_visitors_30_day: f64,
}

// New model for efficient daily visit tracking
#[derive(Debug, Deserialize)]
pub struct DailyVisitRequest {
    pub date: String, // YYYY-MM-DD format
}

// Updated models to match new schema
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DailyStatsFromDb {
    pub id: i32,
    pub date: chrono::NaiveDate,
    pub total_visitors: i32,
    pub unique_visitors: i32,
    pub inheritance_uploads: i32,
    pub support_card_uploads: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct FriendlistReport {
    pub trainer_id: String,
    pub reported_at: DateTime<Utc>,
    pub report_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FriendlistReportResponse {
    pub success: bool,
    pub message: String,
}
