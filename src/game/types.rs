use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountConfig {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    pub credentials: GameCredentials,
    #[serde(default)]
    pub settings: GameSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameCredentials {
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub origin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameSettings {
    #[serde(rename = "huntSlug", default)]
    pub hunt_slug: String,
    #[serde(rename = "autoHunt", default = "default_true")]
    pub auto_hunt: bool,
    #[serde(rename = "autoCatch", default = "default_true")]
    pub auto_catch: bool,
    #[serde(rename = "autoHeal", default = "default_true")]
    pub auto_heal: bool,
    #[serde(rename = "autoDaily", default = "default_true")]
    pub auto_daily: bool,
    #[serde(rename = "autoSpin", default = "default_true")]
    pub auto_spin: bool,
    #[serde(rename = "autoEvolve", default = "default_true")]
    pub auto_evolve: bool,
    #[serde(rename = "ballId", default = "default_ball")]
    pub ball_id: u32,
    #[serde(rename = "shinyBallId", default = "default_shiny_ball")]
    pub shiny_ball_id: u32,
}

fn default_true() -> bool {
    true
}

fn default_ball() -> u32 {
    1
}

fn default_shiny_ball() -> u32 {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrainerData {
    pub name: String,
    #[serde(default)]
    pub level: u32,
    #[serde(default)]
    pub xp: u64,
    #[serde(default)]
    pub hp: u32,
    #[serde(rename = "maxHp", default)]
    pub max_hp: u32,
    #[serde(default)]
    pub gold: i64,
    #[serde(default)]
    pub diamonds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LeaderData {
    #[serde(rename = "pokemonId", default)]
    pub pokemon_id: u64,
    #[serde(rename = "speciesId", default)]
    pub species_id: u32,
    #[serde(default)]
    pub level: u32,
    #[serde(default)]
    pub hp: u32,
    #[serde(rename = "maxHp", default)]
    pub max_hp: u32,
    #[serde(default)]
    pub shiny: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameStateData {
    #[serde(default)]
    pub trainer: TrainerData,
    #[serde(default)]
    pub leader: Option<LeaderData>,
    #[serde(default)]
    pub team: Vec<LeaderData>,
    #[serde(rename = "autoHunt", default)]
    pub auto_hunt: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SnapshotMessage {
    #[serde(rename = "type", default)]
    pub msg_type: String,
    #[serde(default)]
    pub snapshot: Option<GameStateData>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}
