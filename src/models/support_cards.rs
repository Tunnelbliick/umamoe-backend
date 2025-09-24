use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, NaiveDateTime};
use sqlx::FromRow;
use validator::Validate;
use uuid::Uuid;

// Legacy support card models
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct SupportCardRecord {
    pub id: Uuid,
    pub trainer_id: String,
    pub card_id: String,
    pub limit_break: i32,
    pub rarity: i32,
    pub card_type: i32,
    pub submitted_at: DateTime<Utc>,
    pub upvotes: i32,
    pub downvotes: i32,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct SupportCardSearchFilters {
    #[serde(rename = "cardName")]
    pub card_name: Option<String>,
    pub character: Option<String>,
    #[serde(rename = "type")]
    pub card_type: Option<i32>,
    pub rarity: Option<i32>,
    #[serde(rename = "minLimitBreak")]
    pub min_limit_break: Option<i32>,
    #[serde(rename = "maxLimitBreak")]
    pub max_limit_break: Option<i32>,
    #[serde(rename = "sort_by")]
    pub sort_by: Option<String>,
    #[serde(rename = "sort_order")]
    pub sort_order: Option<String>,
}

// V2 Support card models
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct SupportCard {
    pub account_id: String,
    pub support_card_id: i32,
    pub limit_break_count: Option<i32>,
    pub experience: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SupportCardWithTrainer {
    #[serde(flatten)]
    pub support_card: SupportCard,
    pub trainer_name: String,
    pub follower_num: Option<i32>,
    pub last_updated: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SupportCardWithTrainerAndInheritance {
    pub account_id: String,
    pub trainer_name: String,
    pub follower_num: Option<i32>,
    pub last_updated: Option<NaiveDateTime>,
    pub support_cards: Vec<SupportCard>,
    pub inheritance: Option<super::inheritance::Inheritance>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SupportCardMetaData {
    pub record_id: String,
    pub trainer_id: String,
    pub card_name: String,
    pub rarity: i32,
    pub limit_break: i32,
    pub card_type_name: String,
    pub upvotes: i32,
    pub downvotes: i32,
}
