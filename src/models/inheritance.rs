use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, NaiveDateTime};
use sqlx::FromRow;
use validator::Validate;
use uuid::Uuid;
use crate::models::common::{Factor, SkillFactor, deserialize_comma_separated_ints};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct InheritanceRecord {
    pub id: Uuid,
    pub trainer_id: String,
    pub main_character_id: i32,
    pub parent1_id: i32,
    pub parent2_id: i32,
    pub submitted_at: DateTime<Utc>,
    pub verified: bool,
    pub upvotes: i32,
    pub downvotes: i32,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct BlueFactor {
    pub record_id: Uuid,
    pub factor_type: String,
    pub level: i32,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct PinkFactor {
    pub record_id: Uuid,
    pub factor_type: String,
    pub level: i32,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct UniqueSkill {
    pub record_id: Uuid,
    pub skill_id: i32,
    pub level: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InheritanceRecordWithFactors {
    #[serde(flatten)]
    pub record: InheritanceRecord,
    pub blue_factors: Vec<Factor>,
    pub pink_factors: Vec<Factor>,
    pub unique_skills: Vec<SkillFactor>,
}

// V2 Inheritance models
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InheritanceWithTrainer {
    #[serde(flatten)]
    pub inheritance: Inheritance,
    pub trainer_name: String,
    pub follower_num: Option<i32>,
    pub last_updated: Option<NaiveDateTime>,
    pub support_cards: Vec<super::support_cards::SupportCard>,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct InheritanceMetaData {
    pub record_id: String,
    pub trainer_id: String,
    pub character_name: String,
    pub blue_factors_summary: String,
    pub pink_factors_summary: String,
    pub skills_summary: String,
    pub upvotes: i32,
    pub downvotes: i32,
}
