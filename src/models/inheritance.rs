use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Inheritance {
    pub inheritance_id: i32,
    pub account_id: String,
    pub main_parent_id: i32,
    pub parent_left_id: i32,
    pub parent_right_id: i32,
    pub parent_rank: i32,
    pub parent_rarity: i32,
    pub blue_sparks: Vec<i32>,
    pub pink_sparks: Vec<i32>,
    pub green_sparks: Vec<i32>,
    pub white_sparks: Vec<i32>,
    pub win_count: i32,
    pub white_count: i32,
    pub main_blue_factors: i32,
    pub main_pink_factors: i32,
    pub main_green_factors: i32,
    pub main_white_factors: Vec<i32>,
    pub main_white_count: i32,
    #[sqlx(default)]
    pub affinity_score: Option<i32>,
}
