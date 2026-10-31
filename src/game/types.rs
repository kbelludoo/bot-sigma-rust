use serde::{de, Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt;

pub fn deserialize_string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    struct StringOrNumberVisitor;

    impl<'de> de::Visitor<'de> for StringOrNumberVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or an integer")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(v.to_string())
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(v.to_string())
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(v.to_string())
        }
    }

    deserializer.deserialize_any(StringOrNumberVisitor)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "RawAccountConfig")]
pub struct AccountConfig {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub credentials: GameCredentials,
    pub settings: GameSettings,
    pub bind_ip: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawAccountConfig {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default)]
    credentials: Option<GameCredentials>,
    #[serde(default)]
    settings: Option<GameSettings>,

    // Flat fields from Go accounts.json
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    token: Option<String>,
    #[serde(rename = "bindIP", alias = "bind_ip", default)]
    bind_ip: Option<String>,
    #[serde(rename = "huntSlug", alias = "hunt_slug", default)]
    hunt_slug: Option<String>,
    #[serde(rename = "autoHunt", alias = "auto_hunt", default)]
    auto_hunt: Option<bool>,
    #[serde(rename = "autoCatch", alias = "auto_catch", default)]
    auto_catch: Option<bool>,
    #[serde(rename = "autoHeal", alias = "auto_heal", default)]
    auto_heal: Option<bool>,
    #[serde(rename = "autoDaily", alias = "auto_daily", default)]
    auto_daily: Option<bool>,
    #[serde(rename = "autoSpin", alias = "auto_spin", default)]
    auto_spin: Option<bool>,
    #[serde(rename = "autoEvolve", alias = "auto_evolve", default)]
    auto_evolve: Option<bool>,
    #[serde(rename = "ballId", alias = "ball_id", default)]
    ball_id: Option<u32>,
    #[serde(rename = "shinyBallId", alias = "shiny_ball_id", default)]
    shiny_ball_id: Option<u32>,
    #[serde(default)]
    strategy: Option<String>,
    #[serde(rename = "bestiaryTargetKills", alias = "bestiary_target_kills", default)]
    bestiary_target_kills: Option<u32>,
}

impl From<RawAccountConfig> for AccountConfig {
    fn from(raw: RawAccountConfig) -> Self {
        let name = if !raw.name.is_empty() {
            raw.name
        } else {
            raw.id.clone()
        };

        let mut credentials = raw.credentials.unwrap_or_default();
        if credentials.email.is_empty() {
            if let Some(em) = raw.email {
                credentials.email = em;
            }
        }
        if credentials.password.is_empty() {
            if let Some(pw) = raw.password {
                credentials.password = pw;
            }
        }
        if credentials.token.is_empty() {
            if let Some(tk) = raw.token {
                credentials.token = tk;
            }
        }

        let mut settings = raw.settings.unwrap_or_default();
        if let Some(hs) = raw.hunt_slug {
            settings.hunt_slug = hs;
        }
        if let Some(ahunt) = raw.auto_hunt {
            settings.auto_hunt = ahunt;
        }
        if let Some(acatch) = raw.auto_catch {
            settings.auto_catch = acatch;
        }
        if let Some(aheal) = raw.auto_heal {
            settings.auto_heal = aheal;
        }
        if let Some(adaily) = raw.auto_daily {
            settings.auto_daily = adaily;
        }
        if let Some(aspin) = raw.auto_spin {
            settings.auto_spin = aspin;
        }
        if let Some(aevo) = raw.auto_evolve {
            settings.auto_evolve = aevo;
        }
        if let Some(bid) = raw.ball_id {
            settings.ball_id = bid;
        }
        if let Some(sbid) = raw.shiny_ball_id {
            settings.shiny_ball_id = sbid;
        }
        if let Some(strat) = raw.strategy {
            settings.strategy = strat;
        }
        if let Some(btk) = raw.bestiary_target_kills {
            settings.bestiary_target_kills = btk;
        }

        Self {
            id: raw.id,
            name,
            enabled: raw.enabled,
            credentials,
            settings,
            bind_ip: raw.bind_ip,
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(default = "default_strategy")]
    pub strategy: String,
    #[serde(rename = "bestiaryTargetKills", default = "default_bestiary_target")]
    pub bestiary_target_kills: u32,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            hunt_slug: String::new(),
            auto_hunt: true,
            auto_catch: true,
            auto_heal: true,
            auto_daily: true,
            auto_spin: true,
            auto_evolve: true,
            ball_id: default_ball(),
            shiny_ball_id: default_shiny_ball(),
            strategy: default_strategy(),
            bestiary_target_kills: default_bestiary_target(),
        }
    }
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

fn default_strategy() -> String {
    "rush".to_string()
}

fn default_bestiary_target() -> u32 {
    120
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AccountStatus {
    pub id: String,
    pub alias: String,
    pub trainer_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bind_ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outbound_ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
    pub level: u32,
    pub xp: u64,
    pub xp_into_level: u64,
    pub xp_to_next_level: u64,
    pub xp_percent: f64,
    pub hp: u32,
    pub max_hp: u32,
    pub hp_percent: f64,
    pub gold: i64,
    pub diamonds: i64,
    pub vip_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vip_until: Option<i64>,
    pub vip_expiring_soon: bool,
    pub in_game: bool,
    pub in_hunt: bool,
    pub map_slug: String,
    pub map_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leader_species_id: Option<u32>,
    pub leader_name: String,
    pub leader_level: u32,
    pub leader_hp: u32,
    pub leader_max_hp: u32,
    pub leader_shiny: bool,
    pub wild_target: String,
    pub wild_hp: u32,
    pub wild_max_hp: u32,
    pub wild_level: u32,
    pub wild_shiny: bool,
    pub auto_hunt_active: bool,
    pub kills: u64,
    pub captures: u64,
    pub shinies: u64,
    pub gold_found: i64,
    pub trainer_xp_gained: u64,
    pub kills_per_min: f64,
    pub xp_per_hour: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bestiary_level: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bestiary_points: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bestiary_next_points: Option<u32>,
    pub bestiary_target_kills: u32,
    pub bestiary_total_kills: u64,
    pub bestiary_current_kills: u64,
    pub bestiary_completed_count: usize,
    pub bestiary_pending_count: usize,
    pub team_size: usize,
    pub last_action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    pub connected_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InventoryItem {
    #[serde(rename = "itemId", default)]
    pub item_id: u32,
    #[serde(default)]
    pub count: u32,
    #[serde(default)]
    pub category: String,
    #[serde(rename = "sellPrice", default)]
    pub sell_price: u32,
    #[serde(default)]
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SnapshotMessage {
    #[serde(rename = "type", default)]
    pub msg_type: String,
    #[serde(default)]
    pub tick: Option<i64>,
    #[serde(default)]
    pub snapshot: Option<GameStateData>,
    #[serde(default)]
    pub events: Option<Vec<EventData>>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventData {
    #[serde(rename = "type", default)]
    pub event_type: String,
    #[serde(rename = "speciesId", default)]
    pub species_id: u32,
    #[serde(rename = "wildId", default)]
    pub wild_id: Option<u32>,
    #[serde(default)]
    pub shiny: bool,
    #[serde(default)]
    pub gold: i64,
    #[serde(default)]
    pub xp: u64,
    #[serde(default)]
    pub level: u32,
    #[serde(rename = "ballId", default)]
    pub ball_id: Option<u32>,
    #[serde(default)]
    pub success: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameStateData {
    #[serde(default)]
    pub trainer: TrainerData,
    #[serde(default)]
    pub leader: Option<LeaderData>,
    #[serde(default)]
    pub team: Vec<TeamPokemonData>,
    #[serde(default)]
    pub hunt: Option<HuntData>,
    #[serde(default)]
    pub analyzer: Option<AnalyzerData>,
    #[serde(default)]
    pub inventory: Vec<InventoryItem>,
    #[serde(default)]
    pub balls: HashMap<String, u32>,
    #[serde(rename = "boundBalls", default)]
    pub bound_balls: HashMap<String, u32>,
    #[serde(rename = "autoHunt", default)]
    pub auto_hunt: bool,
    #[serde(rename = "inCity", default)]
    pub in_city: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrainerData {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub level: u32,
    #[serde(default)]
    pub xp: u64,
    #[serde(rename = "xpIntoLevel", default)]
    pub xp_into_level: u64,
    #[serde(rename = "xpToNextLevel", default)]
    pub xp_to_next_level: u64,
    #[serde(default)]
    pub hp: u32,
    #[serde(rename = "maxHp", default)]
    pub max_hp: u32,
    #[serde(default)]
    pub gold: i64,
    #[serde(default)]
    pub diamonds: i64,
    #[serde(rename = "vipUntil", default)]
    pub vip_until: Option<i64>,
    #[serde(rename = "vipExpiringSoon", default)]
    pub vip_expiring_soon: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LeaderData {
    #[serde(rename = "pokemonId", deserialize_with = "deserialize_string_or_number", default)]
    pub pokemon_id: String,
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
pub struct TeamPokemonData {
    #[serde(rename = "pokemonId", deserialize_with = "deserialize_string_or_number", default)]
    pub pokemon_id: String,
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
pub struct HuntData {
    #[serde(rename = "mapSlug", default)]
    pub map_slug: String,
    #[serde(rename = "wildSpeciesId", default)]
    pub wild_species_id: u32,
    #[serde(rename = "wildHp", default)]
    pub wild_hp: u32,
    #[serde(rename = "wildMaxHp", default)]
    pub wild_max_hp: u32,
    #[serde(rename = "wildShiny", default)]
    pub wild_shiny: bool,
    #[serde(rename = "wildLevel", default)]
    pub wild_level: u32,
    #[serde(default)]
    pub wilds: Vec<WildData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WildData {
    #[serde(default)]
    pub id: u32,
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
pub struct AnalyzerData {
    #[serde(rename = "elapsedMs", default)]
    pub elapsed_ms: i64,
    #[serde(default)]
    pub kills: u64,
    #[serde(rename = "trainerXp", default)]
    pub trainer_xp: u64,
    #[serde(default)]
    pub captures: u64,
    #[serde(rename = "shinyKills", default)]
    pub shiny_kills: u64,
    #[serde(rename = "shinyCaptures", default)]
    pub shiny_captures: u64,
    #[serde(rename = "lootValue", default)]
    pub loot_value: i64,
    #[serde(rename = "goldFound", default)]
    pub gold_found: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BestiarySpecies {
    #[serde(default)]
    pub id: u32,
    #[serde(default)]
    pub kills: u32,
    #[serde(default)]
    pub level: u32,
    #[serde(default)]
    pub claimed: Vec<u32>,
    #[serde(default)]
    pub owed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BestiaryResponse {
    #[serde(default)]
    pub level: u32,
    #[serde(default)]
    pub points: u32,
    #[serde(rename = "nextPoints", default)]
    pub next_points: Option<u32>,
    #[serde(default)]
    pub species: Vec<BestiarySpecies>,
}
