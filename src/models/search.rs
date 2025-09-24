use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;
use crate::models::common::deserialize_comma_separated_ints;

#[derive(Debug, Serialize)]
pub struct SearchResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
    pub total_pages: i64,
}

// V3 Search API models
#[derive(Debug, Deserialize)]
pub struct UnifiedSearchParams {
    #[serde(default)]
    pub page: Option<i64>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub search_type: Option<String>, // "inheritance", "support_cards", or "all" (default)
    
    // Inheritance filtering
    #[serde(default)]
    pub main_parent_id: Option<i32>,
    #[serde(default)]
    pub parent_left_id: Option<i32>,
    #[serde(default)]
    pub parent_right_id: Option<i32>,
    #[serde(default)]
    pub parent_rank: Option<i32>,
    #[serde(default)]
    pub parent_rarity: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_comma_separated_ints")]
    pub blue_sparks: Option<Vec<i32>>,
    #[serde(default, deserialize_with = "deserialize_comma_separated_ints")]
    pub pink_sparks: Option<Vec<i32>>,
    #[serde(default, deserialize_with = "deserialize_comma_separated_ints")]
    pub green_sparks: Option<Vec<i32>>,
    #[serde(default, deserialize_with = "deserialize_comma_separated_ints")]
    pub white_sparks: Option<Vec<i32>>,
    #[serde(default)]
    pub min_win_count: Option<i32>,
    #[serde(default)]
    pub min_white_count: Option<i32>,
    // Main inherit filtering
    #[serde(default)]
    pub min_main_blue_factors: Option<i32>,
    #[serde(default)]
    pub min_main_pink_factors: Option<i32>,
    #[serde(default)]
    pub min_main_green_factors: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_comma_separated_ints")]
    pub main_white_factors: Option<Vec<i32>>,
    #[serde(default)]
    pub min_main_white_count: Option<i32>,
    
    // Support card filtering
    #[serde(default)]
    pub support_card_id: Option<i32>,
    #[serde(default)]
    pub min_limit_break: Option<i32>,
    #[serde(default)]
    pub max_limit_break: Option<i32>,
    #[serde(default)]
    pub min_experience: Option<i32>,
    
    // Common filtering
    #[serde(default)]
    pub trainer_id: Option<String>, // Direct trainer ID lookup
    #[serde(default)]
    pub max_follower_num: Option<i32>,
    #[serde(default)]
    pub sort_by: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnifiedAccountRecord {
    pub account_id: String,
    pub trainer_name: String,
    pub follower_num: Option<i32>,
    pub last_updated: Option<NaiveDateTime>,
    pub inheritance: Option<super::inheritance::Inheritance>,
    pub support_card: Option<super::support_cards::SupportCard>, // Single best support card, not array
}
