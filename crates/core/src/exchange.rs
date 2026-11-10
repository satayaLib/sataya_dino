use crate::model::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const CSV_COLUMNS: &[&str] = &[
    "schema_version",
    "id",
    "profile_id",
    "profile_name",
    "game",
    "pack_id",
    "pack_version",
    "creature_id",
    "catalog_revision",
    "creature_name",
    "nickname",
    "sex",
    "status",
    "level",
    "tamed_level",
    "stat_stage",
    "health",
    "stamina",
    "oxygen",
    "food",
    "weight",
    "melee_percent",
    "speed_percent",
    "torpor",
    "imprint_percent",
    "map_id",
    "tags_json",
    "extra_stats_json",
    "notes",
    "created_at",
    "updated_at",
];

#[derive(Clone, Debug, Default)]
pub struct CsvOptions {
    pub delimiter: Option<u8>,
    pub decimal_comma: bool,
    pub mapping: BTreeMap<String, String>,
    pub default_profile: Option<Profile>,
    pub default_creature: Option<CreatureDefinition>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImportRow {
    pub line: usize,
    pub value: OwnedCreature,
    pub provided: BTreeSet<String>,
    pub errors: Vec<FieldError>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CsvPreview {
    pub headers: Vec<String>,
    pub delimiter: u8,
    pub rows: Vec<ImportRow>,
    pub profiles: Vec<Profile>,
    pub warnings: Vec<String>,
}

impl CsvPreview {
    pub fn error_count(&self) -> usize {
        self.rows.iter().filter(|r| !r.errors.is_empty()).count()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImportMode {
    Add,
    Merge,
    ReplaceProfile(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImportPlan {
    pub data: AppData,
    pub added: usize,
    pub updated: usize,
    pub skipped: usize,
    pub removed: usize,
}

fn value(c: &OwnedCreature, profile: &Profile, key: &str) -> String {
    match key {
        "schema_version" => "1".into(),
        "id" => c.id.clone(),
        "profile_id" => c.profile_id.clone(),
        "profile_name" => profile.name.clone(),
        "game" => c.reference.game.key().into(),
        "pack_id" => c.reference.pack_id.clone(),
        "pack_version" => c.reference.pack_version.clone(),
        "creature_id" => c.reference.creature_id.clone(),
        "catalog_revision" => c.reference.catalog_revision.clone(),
        "creature_name" => c.reference.creature_name.clone(),
        "nickname" => c.nickname.clone(),
        "sex" => c.sex.clone(),
        "status" => c.status.clone(),
        "level" => c.level.map(|v| v.to_string()).unwrap_or_default(),
        "tamed_level" => c.tamed_level.map(|v| v.to_string()).unwrap_or_default(),
        "stat_stage" => c.stat_stage.clone(),
        "map_id" => c.map_id.clone(),
        "tags_json" => serde_json::to_string(&c.tags).unwrap_or_default(),
        "extra_stats_json" => serde_json::to_string(&c.extra_stats).unwrap_or_default(),
        "notes" => c.notes.clone(),
        "created_at" => c.created_at.clone(),
        "updated_at" => c.updated_at.clone(),
        key => c.stats.get(key).map(|v| v.to_string()).unwrap_or_default(),
    }
}

pub fn export_csv(data: &AppData, creatures: &[OwnedCreature]) -> Result<String, String> {
    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_writer(Vec::new());
    writer
        .write_record(CSV_COLUMNS)
        .map_err(|e| e.to_string())?;
    for c in creatures {
        let profile = data
            .profiles
            .iter()
            .find(|p| p.id == c.profile_id)
            .ok_or("Профиль не найден")?;
        writer
            .write_record(CSV_COLUMNS.iter().map(|key| value(c, profile, key)))
            .map_err(|e| e.to_string())?;
    }
    let bytes = writer.into_inner().map_err(|e| e.to_string())?;
    Ok(format!(
        "\u{feff}{}",
        String::from_utf8(bytes).map_err(|e| e.to_string())?
    ))
}

pub fn parse_decimal(text: &str, decimal_comma: bool) -> Result<Option<f64>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    if text.contains(',') && !decimal_comma {
        return Err("Для дробной запятой выберите соответствующий формат".into());
    }
    if decimal_comma && text.contains('.') && text.contains(',') {
        return Err("Неоднозначные разделители числа".into());
    }
    let normalized = if decimal_comma {
        text.replace(',', ".")
    } else {
        text.to_owned()
    };
    let n = normalized
        .parse::<f64>()
        .map_err(|_| "Некорректное число".to_owned())?;
    if !n.is_finite() || n < 0.0 {
        return Err("Нужно конечное неотрицательное число".into());
    }
    Ok(Some(n))
}

pub fn canonical_header(header: &str) -> String {
    let h = header.trim().to_lowercase().replace(' ', "_");
    match h.as_str() {
        "имя" | "name" => "nickname".into(),
        "вид" | "species" => "creature_name".into(),
        "здоровье" | "hp" => "health".into(),
        "выносливость" => "stamina".into(),
        "вес" => "weight".into(),
        "урон" | "melee" => "melee_percent".into(),
        "уровень" | "lvl" => "level".into(),
        "пол" => "sex".into(),
        "заметка" => "notes".into(),
        "импринт" => "imprint_percent".into(),
        _ => h,
    }
}

pub fn csv_headers(text: &str, delimiter: u8) -> Result<Vec<String>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .from_reader(text.trim_start_matches('\u{feff}').as_bytes());
    reader
        .headers()
        .map(|h| h.iter().map(str::to_owned).collect())
        .map_err(|e| e.to_string())
}

pub fn preview_csv(
    text: &str,
    data: &AppData,
    catalog: &[CreatureDefinition],
    options: &CsvOptions,
    now: &str,
    mut new_id: impl FnMut() -> String,
) -> Result<CsvPreview, String> {
    if text.len() > 20 * 1024 * 1024 {
        return Err("Файл превышает 20 МБ".into());
    }
    let text = text.trim_start_matches('\u{feff}');
    let first = text.lines().next().ok_or("Пустой файл")?;
    let delimiter =
        options
            .delimiter
            .unwrap_or(if first.matches(';').count() > first.matches(',').count() {
                b';'
            } else {
                b','
            });
    if options.decimal_comma && delimiter != b';' {
        return Err("Дробная запятая доступна с разделителем ;".into());
    }
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .trim(csv::Trim::Headers)
        .from_reader(text.as_bytes());
    let headers = reader.headers().map_err(|e| e.to_string())?.clone();
    if headers.is_empty() {
        return Err("Не найдена строка заголовков".into());
    }
    let keys: Vec<String> = headers
        .iter()
        .map(|h| {
            options
                .mapping
                .get(h)
                .cloned()
                .unwrap_or_else(|| canonical_header(h))
        })
        .collect();
    let mut seen = BTreeSet::new();
    for key in &keys {
        if CSV_COLUMNS.contains(&key.as_str()) && !seen.insert(key) {
            return Err(format!("Две колонки сопоставлены полю {key}"));
        }
    }
    let mut preview = CsvPreview {
        headers: headers.iter().map(str::to_owned).collect(),
        delimiter,
        ..Default::default()
    };
    let ignored: Vec<_> = keys
        .iter()
        .filter(|k| !CSV_COLUMNS.contains(&k.as_str()))
        .cloned()
        .collect();
    if !ignored.is_empty() {
        preview
            .warnings
            .push(format!("Пропущенные колонки: {}", ignored.join(", ")));
    }
    let mut ids: BTreeMap<String, usize> = BTreeMap::new();
    let mut line_cursor = 0usize;
    let mut physical_line = 1usize;
    for (index, record) in reader.records().enumerate() {
        if index >= 10000 {
            return Err("Поддерживается не более 10 000 строк".into());
        }
        let record = record.map_err(|e| format!("Ошибка структуры CSV: {e}"))?;
        let mut start = record
            .position()
            .map(|p| p.byte() as usize)
            .unwrap_or(line_cursor);
        let bytes = text.as_bytes();
        while start < bytes.len() && matches!(bytes[start], b'\r' | b'\n') {
            start += 1;
        }
        physical_line += bytes[line_cursor..start]
            .iter()
            .filter(|&&b| b == b'\n')
            .count();
        line_cursor = start;
        let line = physical_line;
        let fields: BTreeMap<_, _> = keys
            .iter()
            .cloned()
            .zip(record.iter().map(str::to_owned))
            .collect();
        let get = |key: &str| fields.get(key).map(String::as_str).unwrap_or("");
        let mut errors = Vec::new();
        if !get("schema_version").is_empty() && get("schema_version") != "1" {
            errors.push(FieldError::new(
                "schema_version",
                "Неподдерживаемая версия CSV",
            ));
        }
        let game = match get("game").trim().to_lowercase().as_str() {
            "ase" => Game::Ase,
            "asa" => Game::Asa,
            "" => options
                .default_profile
                .as_ref()
                .map(|p| p.game)
                .unwrap_or(data.profile().game),
            _ => {
                errors.push(FieldError::new("game", "Допустимо ase или asa"));
                Game::Asa
            }
        };
        let profile_id = if get("profile_id").is_empty() {
            options
                .default_profile
                .as_ref()
                .map(|p| p.id.clone())
                .unwrap_or_else(|| data.profile().id.clone())
        } else {
            get("profile_id").into()
        };
        let profile = data
            .profiles
            .iter()
            .chain(preview.profiles.iter())
            .find(|p| p.id == profile_id)
            .cloned();
        if let Some(p) = profile {
            if p.game != game {
                errors.push(FieldError::new(
                    "profile_id",
                    "Игра не совпадает с профилем",
                ));
            }
        } else {
            let name = if get("profile_name").trim().is_empty() {
                "Импортированный мир"
            } else {
                get("profile_name")
            };
            preview
                .profiles
                .push(Profile::new(profile_id.clone(), name.into(), game));
        }
        let pack_id = if get("pack_id").is_empty() {
            options
                .default_creature
                .as_ref()
                .map(|c| c.pack_id.as_str())
                .unwrap_or("vanilla")
        } else {
            get("pack_id")
        };
        let candidates: Vec<_> = catalog
            .iter()
            .filter(|c| {
                c.game == game
                    && (if !get("creature_id").is_empty() {
                        c.pack_id == pack_id
                            && (c.id == get("creature_id") || c.slug == get("creature_id"))
                    } else {
                        let name = get("creature_name").to_lowercase();
                        !name.is_empty()
                            && (c.name.to_lowercase() == name
                                || c.name_en.to_lowercase() == name
                                || c.aliases.iter().any(|a| a.to_lowercase() == name))
                            && (get("pack_id").is_empty() || c.pack_id == pack_id)
                    })
            })
            .collect();
        let known = if candidates.len() == 1 {
            candidates.first().copied()
        } else if get("creature_id").is_empty() && get("creature_name").is_empty() {
            options.default_creature.as_ref().filter(|c| c.game == game)
        } else {
            None
        };
        if candidates.len() > 1 {
            errors.push(FieldError::new(
                "creature_id",
                "Несколько вариантов вида: укажите ID и пакет",
            ));
        }
        let reference = if let Some(c) = known {
            let mut reference = c.reference();
            if fields.contains_key("pack_version") {
                reference.pack_version = get("pack_version").into();
            }
            if fields.contains_key("catalog_revision") {
                reference.catalog_revision = get("catalog_revision").into();
            }
            if fields.contains_key("creature_name") && !get("creature_name").is_empty() {
                reference.creature_name = get("creature_name").into();
            }
            reference
        } else {
            if get("creature_id").is_empty() {
                errors.push(FieldError::new(
                    "creature_id",
                    "Выберите вид или укажите его стабильный ID",
                ));
            }
            CreatureRef {
                game,
                creature_id: get("creature_id").into(),
                pack_id: pack_id.into(),
                pack_version: get("pack_version").into(),
                catalog_revision: get("catalog_revision").into(),
                creature_name: get("creature_name").into(),
            }
        };
        let id = if get("id").is_empty() {
            new_id()
        } else {
            get("id").into()
        };
        let mut c = OwnedCreature::new(id.clone(), profile_id, reference, now.into());
        for key in [
            "nickname",
            "sex",
            "status",
            "stat_stage",
            "map_id",
            "notes",
            "created_at",
            "updated_at",
        ] {
            let v = get(key);
            match key {
                "nickname" => c.nickname = v.into(),
                "sex" if !v.is_empty() => c.sex = v.to_lowercase(),
                "status" if !v.is_empty() => c.status = v.into(),
                "stat_stage" if !v.is_empty() => c.stat_stage = v.into(),
                "map_id" => c.map_id = v.into(),
                "notes" => c.notes = v.into(),
                "created_at" if !v.is_empty() => c.created_at = v.into(),
                "updated_at" if !v.is_empty() => c.updated_at = v.into(),
                _ => (),
            }
        }
        for key in ["level", "tamed_level"] {
            let parsed = if get(key).trim().is_empty() {
                None
            } else {
                match get(key).trim().parse::<u32>() {
                    Ok(v) if v > 0 => Some(v),
                    _ => {
                        errors.push(FieldError::new(key, "Нужно положительное целое"));
                        None
                    }
                }
            };
            if key == "level" {
                c.level = parsed;
            } else {
                c.tamed_level = parsed;
            }
        }
        for (key, _, _) in STAT_FIELDS {
            match parse_decimal(get(key), options.decimal_comma) {
                Ok(v) => c.stats.set(key, v),
                Err(message) => errors.push(FieldError {
                    field: key.into(),
                    message,
                }),
            }
        }
        if !get("tags_json").is_empty() {
            match serde_json::from_str(get("tags_json")) {
                Ok(tags) => c.tags = tags,
                Err(_) => errors.push(FieldError::new("tags_json", "Нужен JSON-массив строк")),
            }
        }
        if !get("extra_stats_json").is_empty() {
            match serde_json::from_str(get("extra_stats_json")) {
                Ok(stats) => c.extra_stats = stats,
                Err(_) => errors.push(FieldError::new(
                    "extra_stats_json",
                    "Нужен JSON-объект числовых статов",
                )),
            }
        }
        errors.extend(c.validate());
        if let Some(previous) = ids.insert(id, preview.rows.len()) {
            errors.push(FieldError::new("id", "UUID повторяется внутри файла"));
            preview.rows[previous]
                .errors
                .push(FieldError::new("id", "UUID повторяется внутри файла"));
        }
        preview.rows.push(ImportRow {
            line,
            value: c,
            provided: fields.keys().cloned().collect(),
            errors,
        });
    }
    if preview.rows.is_empty() {
        return Err("В CSV нет записей".into());
    }
    if !keys.iter().any(|k| k == "id") {
        preview.warnings.push("UUID не заданы: для новых строк созданы ID. Повторная загрузка исходного файла создаст новые записи.".into());
    }
    Ok(preview)
}

fn merge_row(old: &OwnedCreature, row: &ImportRow, now: &str) -> Result<OwnedCreature, String> {
    let mut target = serde_json::to_value(old).map_err(|e| e.to_string())?;
    let source = serde_json::to_value(&row.value).map_err(|e| e.to_string())?;
    for key in &row.provided {
        if STAT_FIELDS.iter().any(|(k, _, _)| k == key) {
            target["stats"][key] = source["stats"][key].clone();
        } else {
            let field = match key.as_str() {
                "tags_json" => "tags",
                "extra_stats_json" => "extra_stats",
                key => key,
            };
            if let Some(v) = source.get(field) {
                target[field] = v.clone();
            }
        }
    }
    let mut merged: OwnedCreature = serde_json::from_value(target).map_err(|e| e.to_string())?;
    if merged != *old && !row.provided.contains("updated_at") {
        merged.updated_at = now.into();
    }
    Ok(merged)
}

pub fn plan_csv_import(
    data: &AppData,
    preview: &CsvPreview,
    mode: &ImportMode,
    skip_errors: bool,
    now: &str,
) -> Result<ImportPlan, String> {
    if preview.error_count() > 0 && !skip_errors {
        return Err("Исправьте ошибки или явно выберите импорт корректных строк".into());
    }
    let mut next = data.clone();
    let mut plan = ImportPlan {
        data: data.clone(),
        added: 0,
        updated: 0,
        skipped: 0,
        removed: 0,
    };
    let target = if let ImportMode::ReplaceProfile(id) = mode {
        let target = next
            .profiles
            .iter()
            .find(|p| &p.id == id)
            .cloned()
            .ok_or("Профиль замены не найден")?;
        let valid: Vec<_> = preview
            .rows
            .iter()
            .filter(|r| r.errors.is_empty())
            .collect();
        if valid.is_empty() {
            return Err("Нет корректных строк для замены профиля".into());
        }
        if valid.iter().any(|r| r.value.reference.game != target.game) {
            return Err("Игра в файле не соответствует профилю замены".into());
        }
        if valid.iter().any(|r| {
            data.creatures
                .iter()
                .any(|c| c.id == r.value.id && c.profile_id != *id)
        }) {
            return Err(
                "UUID из файла принадлежит другому профилю: замена затронула бы чужие записи"
                    .into(),
            );
        }
        plan.removed = next
            .creatures
            .iter()
            .filter(|c| c.profile_id == *id)
            .count();
        next.creatures.retain(|c| c.profile_id != *id);
        Some(target)
    } else {
        None
    };
    let mut indices: BTreeMap<String, usize> = next
        .creatures
        .iter()
        .enumerate()
        .map(|(i, c)| (c.id.clone(), i))
        .collect();
    for row in &preview.rows {
        if !row.errors.is_empty() {
            plan.skipped += 1;
            continue;
        }
        let mut c = row.value.clone();
        if let Some(target) = &target {
            c.profile_id = target.id.clone();
        }
        if let Some(&index) = indices.get(&c.id) {
            if *mode == ImportMode::Add {
                plan.skipped += 1;
                continue;
            }
            let old = &next.creatures[index];
            if old.reference.game != c.reference.game
                || old.reference.pack_id != c.reference.pack_id
                || old.reference.creature_id != c.reference.creature_id
            {
                return Err(format!(
                    "Строка {}: UUID {} относится к другому виду или игре",
                    row.line, c.id
                ));
            }
            let merged = merge_row(old, row, now)?;
            if merged == *old {
                plan.skipped += 1;
            } else {
                next.creatures[index] = merged;
                plan.updated += 1;
            }
        } else {
            indices.insert(c.id.clone(), next.creatures.len());
            next.creatures.push(c);
            plan.added += 1;
        }
    }
    for profile in &preview.profiles {
        if next.creatures.iter().any(|c| c.profile_id == profile.id)
            && !next.profiles.iter().any(|p| p.id == profile.id)
        {
            next.profiles.push(profile.clone());
        }
    }
    next.validate()?;
    plan.data = next;
    Ok(plan)
}

pub fn export_errors(preview: &CsvPreview) -> String {
    let mut writer = csv::Writer::from_writer(Vec::new());
    let _ = writer.write_record(["line", "id", "field", "error"]);
    for row in &preview.rows {
        for error in &row.errors {
            let _ = writer.write_record([
                row.line.to_string(),
                row.value.id.clone(),
                error.field.clone(),
                error.message.clone(),
            ]);
        }
    }
    String::from_utf8(writer.into_inner().unwrap_or_default()).unwrap_or_default()
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Backup {
    pub format: String,
    pub schema_version: u32,
    pub app_version: String,
    pub exported_at: String,
    pub data: AppData,
}

pub fn export_backup(data: &AppData, now: &str) -> Result<String, String> {
    data.validate()?;
    serde_json::to_string_pretty(&Backup {
        format: "sataya-ark-backup".into(),
        schema_version: 1,
        app_version: env!("CARGO_PKG_VERSION").into(),
        exported_at: now.into(),
        data: data.clone(),
    })
    .map_err(|e| e.to_string())
}

pub fn parse_backup(text: &str) -> Result<Backup, String> {
    if text.len() > 20 * 1024 * 1024 {
        return Err("Файл превышает 20 МБ".into());
    }
    let backup: Backup = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("Некорректный JSON: {e}"))?;
    if backup.format != "sataya-ark-backup" || backup.schema_version != 1 {
        return Err("Неподдерживаемый формат или версия резервной копии".into());
    }
    backup.data.validate()?;
    Ok(backup)
}

pub fn plan_backup(data: &AppData, backup: &Backup, replace: bool) -> Result<ImportPlan, String> {
    let mut next = if replace {
        backup.data.clone()
    } else {
        data.clone()
    };
    let mut added = 0;
    let mut updated = 0;
    if replace {
        added = next.creatures.len();
    } else {
        for p in &backup.data.profiles {
            if let Some(old) = next.profiles.iter_mut().find(|x| x.id == p.id) {
                if old.game != p.game {
                    return Err("Конфликт игры для UUID профиля".into());
                }
                *old = p.clone();
            } else {
                next.profiles.push(p.clone());
            }
        }
        for c in &backup.data.creatures {
            if let Some(old) = next.creatures.iter_mut().find(|x| x.id == c.id) {
                if old.reference.game != c.reference.game
                    || old.reference.creature_id != c.reference.creature_id
                    || old.reference.pack_id != c.reference.pack_id
                {
                    return Err("Конфликт вида для UUID дино".into());
                }
                if old != c {
                    *old = c.clone();
                    updated += 1;
                }
            } else {
                next.creatures.push(c.clone());
                added += 1;
            }
        }
        next.preferences
            .favorites
            .extend(backup.data.preferences.favorites.iter().cloned());
        next.preferences
            .wishlist
            .extend(backup.data.preferences.wishlist.iter().cloned());
    }
    next.revision = data.revision;
    next.validate()?;
    Ok(ImportPlan {
        data: next,
        added,
        updated,
        skipped: 0,
        removed: if replace { data.creatures.len() } else { 0 },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const NOW: &str = "2026-11-10T13:00:00Z";
    fn sample() -> AppData {
        let mut d = AppData::default();
        let mut c = OwnedCreature::new(
            "11111111-1111-4111-8111-111111111111".into(),
            d.profile().id.clone(),
            CreatureRef {
                creature_id: "asa:vanilla:rex".into(),
                game: Game::Asa,
                pack_id: "vanilla".into(),
                pack_version: "1".into(),
                catalog_revision: "1".into(),
                creature_name: "Рекс".into(),
            },
            NOW.into(),
        );
        c.nickname = "Борис, \"Лучший\"".into();
        c.notes = "Первая строка\nВторая, с кавычкой: \"".into();
        c.level = Some(900);
        c.stats.health = Some(0.0);
        c.stats.melee_percent = Some(351.75);
        c.tags = vec!["босс".into(), "пара".into()];
        c.extra_stats.insert("charge".into(), 12.5);
        c.set_point(PointStage::BeforeTame, "health", Some(29));
        c.set_point(PointStage::AfterTame, "health", Some(40));
        c.set_point(PointStage::Mutations, "health", Some(0));
        c.set_point(PointStage::Levelups, "health", Some(10));
        d.creatures.push(c);
        d
    }
    fn parse(text: &str, data: &AppData) -> CsvPreview {
        preview_csv(text, data, &[], &CsvOptions::default(), NOW, || {
            "22222222-2222-4222-8222-222222222222".into()
        })
        .unwrap()
    }
    #[test]
    fn csv_round_trip_preserves_all_values_and_missing_stats() {
        let data = sample();
        let csv = export_csv(&data, &data.creatures).unwrap();
        let empty = AppData::default();
        let preview = parse(&csv, &empty);
        assert_eq!(preview.error_count(), 0);
        assert_eq!(
            plan_csv_import(&empty, &preview, &ImportMode::Merge, false, NOW)
                .unwrap()
                .data
                .creatures,
            data.creatures
        );
    }
    #[test]
    fn repeated_merge_is_idempotent() {
        let data = sample();
        let csv = export_csv(&data, &data.creatures).unwrap();
        let plan =
            plan_csv_import(&data, &parse(&csv, &data), &ImportMode::Merge, false, NOW).unwrap();
        assert_eq!(plan.updated, 0);
        assert_eq!(plan.skipped, 1);
        assert_eq!(plan.data, data);
    }

    #[test]
    fn point_snapshots_reject_fractional_values_and_preserve_zero() {
        let mut data = sample();
        assert_eq!(
            data.creatures[0].point(PointStage::Mutations, "health"),
            Some(0)
        );
        data.creatures[0]
            .extra_stats
            .insert("points.after_tame.health".into(), 29.5);
        assert!(!data.creatures[0].validate().is_empty());
        let csv = export_csv(&data, &data.creatures).unwrap();
        let empty = AppData::default();
        let preview = parse(&csv, &empty);
        assert_eq!(preview.error_count(), 1);
        assert!(plan_csv_import(&empty, &preview, &ImportMode::Merge, false, NOW).is_err());
    }

    #[test]
    fn csv_error_positions_include_header_and_quoted_newlines() {
        let data = sample();
        let mut other = data.creatures[0].clone();
        other.id = "22222222-2222-4222-8222-222222222222".into();
        let csv = export_csv(&data, &[data.creatures[0].clone(), other]).unwrap();
        let preview = parse(&csv, &data);
        assert_eq!(preview.rows[0].line, 2);
        assert_eq!(preview.rows[1].line, 4);
    }
    #[test]
    fn add_never_overwrites_existing_uuid() {
        let data = sample();
        let mut incoming = data.clone();
        incoming.creatures[0].nickname = "Другое имя".into();
        let p = parse(&export_csv(&incoming, &incoming.creatures).unwrap(), &data);
        assert_eq!(
            plan_csv_import(&data, &p, &ImportMode::Add, false, NOW)
                .unwrap()
                .data,
            data
        );
    }
    #[test]
    fn duplicate_uuid_marks_both_rows() {
        let data = sample();
        let csv = export_csv(
            &data,
            &[data.creatures[0].clone(), data.creatures[0].clone()],
        )
        .unwrap();
        let preview = parse(&csv, &data);
        assert_eq!(preview.error_count(), 2);
        assert!(plan_csv_import(&data, &preview, &ImportMode::Merge, false, NOW).is_err());
    }
    #[test]
    fn absent_columns_preserve_values_and_empty_cells_clear_optional_values() {
        let data = sample();
        let c = &data.creatures[0];
        let csv = format!(
            "id,creature_id,creature_name,health\n{},asa:vanilla:rex,Рекс,\n",
            c.id
        );
        let plan =
            plan_csv_import(&data, &parse(&csv, &data), &ImportMode::Merge, false, NOW).unwrap();
        assert_eq!(plan.data.creatures[0].stats.health, None);
        assert_eq!(
            plan.data.creatures[0].stats.melee_percent,
            c.stats.melee_percent
        );
        assert_eq!(plan.data.creatures[0].nickname, c.nickname);
    }
    #[test]
    fn malformed_numbers_are_reported_before_writes() {
        let data = sample();
        let csv = export_csv(&data, &data.creatures)
            .unwrap()
            .replace(",351.75,", ",NaN,");
        let p = parse(&csv, &data);
        assert_eq!(p.error_count(), 1);
        assert!(plan_csv_import(&data, &p, &ImportMode::Merge, false, NOW).is_err());
    }
    #[test]
    fn duplicate_headers_are_rejected() {
        assert!(
            preview_csv(
                "id,id\na,b",
                &AppData::default(),
                &[],
                &CsvOptions::default(),
                NOW,
                String::new
            )
            .is_err()
        );
    }
    #[test]
    fn replacement_keeps_other_profiles() {
        let mut data = sample();
        let mut second = data.creatures[0].clone();
        second.id = "22222222-2222-4222-8222-222222222222".into();
        second.profile_id = data.profiles[1].id.clone();
        second.reference.game = Game::Ase;
        second.reference.creature_id = "ase:vanilla:rex".into();
        data.creatures.push(second.clone());
        let csv = export_csv(&data, &data.creatures[..1]).unwrap();
        let plan = plan_csv_import(
            &data,
            &parse(&csv, &data),
            &ImportMode::ReplaceProfile(data.profile().id.clone()),
            false,
            NOW,
        )
        .unwrap();
        assert!(plan.data.creatures.contains(&second));
        assert_eq!(plan.data.creatures.len(), 2);
    }
    #[test]
    fn replacement_rejects_uuid_owned_by_another_profile() {
        let mut data = sample();
        let mut p = Profile::new(
            "33333333-3333-4333-8333-333333333333".into(),
            "Другой".into(),
            Game::Asa,
        );
        p.map.clear();
        data.profiles.push(p.clone());
        let csv = export_csv(&data, &data.creatures).unwrap();
        assert!(
            plan_csv_import(
                &data,
                &parse(&csv, &data),
                &ImportMode::ReplaceProfile(p.id),
                false,
                NOW
            )
            .is_err()
        );
    }
    #[test]
    fn unknown_mod_survives_csv_and_backup() {
        let mut data = sample();
        data.creatures[0].reference.pack_id = "missing-mod".into();
        let p = parse(
            &export_csv(&data, &data.creatures).unwrap(),
            &AppData::default(),
        );
        assert_eq!(p.rows[0].value.reference.pack_id, "missing-mod");
        let backup = parse_backup(&export_backup(&data, NOW).unwrap()).unwrap();
        assert_eq!(backup.data, data);
    }
    #[test]
    fn backup_restores_preferences_and_rejects_future_versions() {
        let mut data = sample();
        data.preferences.theme = "dark".into();
        data.preferences.wishlist.insert("asa:vanilla:rex".into());
        let text = export_backup(&data, NOW).unwrap();
        let b = parse_backup(&text).unwrap();
        assert_eq!(
            plan_backup(&AppData::default(), &b, true).unwrap().data,
            data
        );
        assert!(
            parse_backup(&text.replacen("\"schema_version\": 1", "\"schema_version\": 9", 1))
                .is_err()
        );
    }
    #[test]
    fn decimal_comma_requires_explicit_format() {
        assert!(parse_decimal("12,5", false).is_err());
        assert_eq!(parse_decimal("12,5", true).unwrap(), Some(12.5));
        assert!(parse_decimal("1.000,5", true).is_err());
        assert!(parse_decimal("-1", false).is_err());
    }
    #[test]
    fn invalid_profile_rates_are_rejected() {
        let mut data = sample();
        data.profiles[0].taming_multiplier = 0.0;
        assert!(data.validate().is_err());
    }
}
