use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Game {
    Ase,
    #[default]
    Asa,
}

impl Game {
    pub fn key(self) -> &'static str {
        match self {
            Self::Ase => "ase",
            Self::Asa => "asa",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Ase => "Survival Evolved",
            Self::Asa => "Survival Ascended",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ContentPack {
    pub id: String,
    pub name: String,
    pub game: Game,
    pub version: String,
    pub source: String,
    pub description: String,
    pub available: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Guide {
    pub method: String,
    pub food: Vec<String>,
    pub equipment: Vec<String>,
    pub steps: Vec<String>,
    pub notes: Vec<String>,
    pub source: String,
    pub checked_at: String,
    #[serde(default)]
    pub partial: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CreatureDefinition {
    pub id: String,
    pub slug: String,
    pub game: Game,
    pub pack_id: String,
    pub pack_version: String,
    pub revision: String,
    pub name: String,
    pub name_en: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub group: String,
    pub diet: String,
    pub role: String,
    pub description: String,
    pub art: String,
    pub maps: Vec<String>,
    pub guide: Guide,
}

impl CreatureDefinition {
    pub fn matches(&self, query: &str) -> bool {
        let q = query.to_lowercase();
        self.name.to_lowercase().contains(&q)
            || self.name_en.to_lowercase().contains(&q)
            || self.aliases.iter().any(|a| a.to_lowercase().contains(&q))
    }
    pub fn reference(&self) -> CreatureRef {
        CreatureRef {
            creature_id: self.id.clone(),
            game: self.game,
            pack_id: self.pack_id.clone(),
            pack_version: self.pack_version.clone(),
            catalog_revision: self.revision.clone(),
            creature_name: self.name.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Stats {
    pub health: Option<f64>,
    pub stamina: Option<f64>,
    pub oxygen: Option<f64>,
    pub food: Option<f64>,
    pub weight: Option<f64>,
    pub melee_percent: Option<f64>,
    pub speed_percent: Option<f64>,
    pub torpor: Option<f64>,
    pub imprint_percent: Option<f64>,
}

pub const STAT_FIELDS: [(&str, &str, &str); 9] = [
    ("health", "Здоровье", "HP"),
    ("stamina", "Выносливость", ""),
    ("oxygen", "Кислород", ""),
    ("food", "Еда", ""),
    ("weight", "Вес", ""),
    ("melee_percent", "Урон", "%"),
    ("speed_percent", "Скорость", "%"),
    ("torpor", "Оглушение", ""),
    ("imprint_percent", "Импринт", "%"),
];

impl Stats {
    pub fn get(&self, key: &str) -> Option<f64> {
        match key {
            "health" => self.health,
            "stamina" => self.stamina,
            "oxygen" => self.oxygen,
            "food" => self.food,
            "weight" => self.weight,
            "melee_percent" => self.melee_percent,
            "speed_percent" => self.speed_percent,
            "torpor" => self.torpor,
            "imprint_percent" => self.imprint_percent,
            _ => None,
        }
    }
    pub fn set(&mut self, key: &str, value: Option<f64>) {
        match key {
            "health" => self.health = value,
            "stamina" => self.stamina = value,
            "oxygen" => self.oxygen = value,
            "food" => self.food = value,
            "weight" => self.weight = value,
            "melee_percent" => self.melee_percent = value,
            "speed_percent" => self.speed_percent = value,
            "torpor" => self.torpor = value,
            "imprint_percent" => self.imprint_percent = value,
            _ => (),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CreatureRef {
    pub creature_id: String,
    pub game: Game,
    pub pack_id: String,
    pub pack_version: String,
    pub catalog_revision: String,
    pub creature_name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct OwnedCreature {
    pub id: String,
    pub profile_id: String,
    #[serde(flatten)]
    pub reference: CreatureRef,
    pub nickname: String,
    pub sex: String,
    pub status: String,
    pub level: Option<u32>,
    pub tamed_level: Option<u32>,
    pub stat_stage: String,
    pub stats: Stats,
    pub map_id: String,
    pub tags: Vec<String>,
    pub extra_stats: BTreeMap<String, f64>,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointStage {
    BeforeTame,
    AfterTame,
    Mutations,
    Levelups,
}

pub const POINT_STAGES: [(PointStage, &str); 4] = [
    (PointStage::BeforeTame, "До приручения"),
    (PointStage::AfterTame, "База после приручения"),
    (PointStage::Mutations, "Мутации"),
    (PointStage::Levelups, "Прокачка"),
];

impl PointStage {
    pub fn key(self) -> &'static str {
        match self {
            Self::BeforeTame => "before_tame",
            Self::AfterTame => "after_tame",
            Self::Mutations => "mutations",
            Self::Levelups => "levelups",
        }
    }
    pub fn from_key(key: &str) -> Option<Self> {
        POINT_STAGES
            .iter()
            .find(|(stage, _)| stage.key() == key)
            .map(|(stage, _)| *stage)
    }
    pub fn field_key(self, stat: &str) -> String {
        format!("points.{}.{stat}", self.key())
    }
}

impl OwnedCreature {
    pub fn point(&self, stage: PointStage, stat: &str) -> Option<u32> {
        self.extra_stats
            .get(&stage.field_key(stat))
            .copied()
            .filter(|value| {
                value.is_finite()
                    && *value >= 0.0
                    && *value <= u32::MAX as f64
                    && value.fract() == 0.0
            })
            .map(|value| value as u32)
    }

    pub fn set_point(&mut self, stage: PointStage, stat: &str, value: Option<u32>) {
        let key = stage.field_key(stat);
        if let Some(value) = value {
            self.extra_stats.insert(key, value as f64);
        } else {
            self.extra_stats.remove(&key);
        }
    }
    pub fn new(id: String, profile_id: String, reference: CreatureRef, now: String) -> Self {
        Self {
            id,
            profile_id,
            reference,
            nickname: String::new(),
            sex: "unknown".into(),
            status: "active".into(),
            level: None,
            tamed_level: None,
            stat_stage: "current".into(),
            stats: Stats::default(),
            map_id: String::new(),
            tags: vec![],
            extra_stats: BTreeMap::new(),
            notes: String::new(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
    pub fn title(&self) -> &str {
        if self.nickname.trim().is_empty() {
            &self.reference.creature_name
        } else {
            &self.nickname
        }
    }
    pub fn validate(&self) -> Vec<FieldError> {
        let mut errors = Vec::new();
        for (key, value) in [("id", &self.id), ("profile_id", &self.profile_id)] {
            if uuid::Uuid::parse_str(value).is_err() {
                errors.push(FieldError::new(key, "Нужен UUID"));
            }
        }
        for (key, value) in [
            ("creature_id", &self.reference.creature_id),
            ("pack_id", &self.reference.pack_id),
            ("creature_name", &self.reference.creature_name),
        ] {
            if value.trim().is_empty() {
                errors.push(FieldError::new(key, "Обязательное поле"));
            }
        }
        if !["male", "female", "unknown"].contains(&self.sex.as_str()) {
            errors.push(FieldError::new("sex", "Допустимо male, female или unknown"));
        }
        if !["active", "cryopod", "dead", "transferred"].contains(&self.status.as_str()) {
            errors.push(FieldError::new("status", "Неизвестный статус"));
        }
        if !["current", "after_tame", "newborn"].contains(&self.stat_stage.as_str()) {
            errors.push(FieldError::new(
                "stat_stage",
                "Неизвестная стадия измерения",
            ));
        }
        for (key, value) in [("level", self.level), ("tamed_level", self.tamed_level)] {
            if value == Some(0) {
                errors.push(FieldError::new(key, "Уровень должен быть больше нуля"));
            }
        }
        for (key, _, _) in STAT_FIELDS {
            if self
                .stats
                .get(key)
                .is_some_and(|n| !n.is_finite() || n < 0.0)
            {
                errors.push(FieldError::new(key, "Нужно конечное неотрицательное число"));
            }
        }
        for (key, value) in &self.extra_stats {
            if key.starts_with("points.") {
                let parts: Vec<_> = key.split('.').collect();
                if parts.len() != 3
                    || PointStage::from_key(parts[1]).is_none()
                    || !STAT_FIELDS
                        .iter()
                        .any(|(stat, _, _)| *stat == parts[2] && *stat != "imprint_percent")
                    || value.fract() != 0.0
                    || *value > u32::MAX as f64
                {
                    errors.push(FieldError::new("extra_stats_json", "Очки характеристик должны быть целыми неотрицательными числами с указанной стадией"));
                }
            }
            if key.trim().is_empty() || !value.is_finite() || *value < 0.0 {
                errors.push(FieldError::new(
                    "extra_stats_json",
                    "Некорректные дополнительные статы",
                ));
            }
        }
        for (key, value) in [
            ("created_at", &self.created_at),
            ("updated_at", &self.updated_at),
        ] {
            if chrono::DateTime::parse_from_rfc3339(value).is_err() {
                errors.push(FieldError::new(key, "Нужна дата ISO 8601 с часовым поясом"));
            }
        }
        if self.nickname.len() > 500 || self.notes.len() > 50000 {
            errors.push(FieldError::new("notes", "Слишком длинный текст"));
        }
        errors
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub game: Game,
    pub map: String,
    pub packs: BTreeSet<String>,
    pub taming_multiplier: f64,
    pub food_multiplier: f64,
    pub stat_multiplier: f64,
    pub single_player: bool,
}

impl Profile {
    pub fn new(id: String, name: String, game: Game) -> Self {
        Self {
            id,
            name,
            game,
            map: "The Island".into(),
            packs: BTreeSet::from(["vanilla".into()]),
            taming_multiplier: 1.0,
            food_multiplier: 1.0,
            stat_multiplier: 1.0,
            single_player: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Preferences {
    pub active_profile_id: String,
    pub favorites: BTreeSet<String>,
    pub wishlist: BTreeSet<String>,
    pub theme: String,
    pub last_export_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct AppData {
    pub schema_version: u32,
    pub revision: u64,
    pub profiles: Vec<Profile>,
    pub creatures: Vec<OwnedCreature>,
    pub preferences: Preferences,
}

impl Default for AppData {
    fn default() -> Self {
        let asa = Profile::new(
            "00000000-0000-4000-8000-000000000001".into(),
            "Мой мир · ASA".into(),
            Game::Asa,
        );
        let ase = Profile::new(
            "00000000-0000-4000-8000-000000000002".into(),
            "Мой мир · ASE".into(),
            Game::Ase,
        );
        Self {
            schema_version: 1,
            revision: 0,
            preferences: Preferences {
                active_profile_id: asa.id.clone(),
                favorites: BTreeSet::new(),
                wishlist: BTreeSet::new(),
                theme: "light".into(),
                last_export_at: None,
            },
            profiles: vec![asa, ase],
            creatures: vec![],
        }
    }
}

impl AppData {
    pub fn profile(&self) -> &Profile {
        self.profiles
            .iter()
            .find(|p| p.id == self.preferences.active_profile_id)
            .unwrap_or(&self.profiles[0])
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("Неподдерживаемая версия данных".into());
        }
        if self.profiles.is_empty() {
            return Err("Нужен хотя бы один профиль".into());
        }
        let mut ids = BTreeSet::new();
        for p in &self.profiles {
            if uuid::Uuid::parse_str(&p.id).is_err()
                || p.name.trim().is_empty()
                || !ids.insert(&p.id)
            {
                return Err("Некорректный профиль или повторяющийся ID".into());
            }
            if [p.taming_multiplier, p.food_multiplier, p.stat_multiplier]
                .iter()
                .any(|v| !v.is_finite() || *v <= 0.0)
            {
                return Err("Множители профиля должны быть положительными числами".into());
            }
        }
        if !ids.contains(&self.preferences.active_profile_id) {
            return Err("Активный профиль не найден".into());
        }
        let mut creature_ids = BTreeSet::new();
        for c in &self.creatures {
            if !creature_ids.insert(&c.id) {
                return Err(format!("Повторяющийся UUID: {}", c.id));
            }
            if let Some(error) = c.validate().first() {
                return Err(format!(
                    "{}: {} — {}",
                    c.title(),
                    error.field,
                    error.message
                ));
            }
            let p = self
                .profiles
                .iter()
                .find(|p| p.id == c.profile_id)
                .ok_or("Профиль дино не найден")?;
            if p.game != c.reference.game {
                return Err("Игра дино не совпадает с профилем".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

impl FieldError {
    pub fn new(field: &str, message: &str) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

pub fn number(value: Option<f64>) -> String {
    match value {
        Some(n) if n.fract() == 0.0 => format!("{n:.0}"),
        Some(n) => n.to_string(),
        None => "—".into(),
    }
}
