use crate::model::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct OcrLine {
    pub text: String,
    #[serde(default)]
    pub confidence: f64,
    #[serde(default)]
    pub bbox: Option<[f64; 4]>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct OcrDocument {
    pub text: String,
    #[serde(default)]
    pub lines: Vec<OcrLine>,
    #[serde(default)]
    pub layout: String,
    #[serde(default)]
    pub regions: BTreeMap<String, OcrLine>,
    #[serde(default)]
    pub portrait_family: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct OcrEvidence {
    pub field: String,
    pub raw: String,
    pub value: Option<f64>,
    pub confidence: f64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct OcrDraft {
    pub nickname: Option<String>,
    pub level: Option<u32>,
    pub tamed_level: Option<u32>,
    pub sex: Option<String>,
    pub stats: Stats,
    pub point_values: BTreeMap<String, u32>,
    pub candidates: Vec<String>,
    pub portrait_family: Option<String>,
    pub evidence: Vec<OcrEvidence>,
    pub warnings: Vec<String>,
    pub raw_text: String,
}

fn grouped_lines(document: &OcrDocument) -> Vec<OcrLine> {
    if document.lines.is_empty() {
        return document
            .text
            .lines()
            .map(|text| OcrLine {
                text: text.into(),
                confidence: 0.0,
                bbox: None,
            })
            .collect();
    }
    let mut sorted = document.lines.clone();
    sorted.sort_by(|a, b| {
        let a = a.bbox.unwrap_or_default();
        let b = b.bbox.unwrap_or_default();
        a[1].total_cmp(&b[1]).then(a[0].total_cmp(&b[0]))
    });
    let mut rows: Vec<Vec<OcrLine>> = vec![];
    for line in sorted {
        let matching = rows.iter().position(|row| match (row[0].bbox, line.bbox) {
            (Some(a), Some(b)) => {
                ((a[1] + a[3]) / 2.0 - (b[1] + b[3]) / 2.0).abs()
                    < ((a[3] - a[1]).min(b[3] - b[1]) * 0.45)
            }
            _ => false,
        });
        if let Some(i) = matching {
            rows[i].push(line);
        } else {
            rows.push(vec![line]);
        }
    }
    rows.into_iter()
        .map(|mut row| {
            row.sort_by(|a, b| {
                a.bbox.unwrap_or_default()[0].total_cmp(&b.bbox.unwrap_or_default()[0])
            });
            OcrLine {
                text: row
                    .iter()
                    .map(|r| r.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
                confidence: row.iter().map(|r| r.confidence).fold(100.0, f64::min),
                bbox: row[0].bbox,
            }
        })
        .collect()
}

fn ocr_number(text: &str) -> Result<f64, String> {
    let text = text.trim();
    let mut compact = text.replace([' ', '\u{a0}'], "");
    let multiplier = match compact.chars().last() {
        Some('k' | 'K' | 'к' | 'К') => 1000.0,
        Some('m' | 'M' | 'м' | 'М') => 1_000_000.0,
        _ => 1.0,
    };
    if multiplier != 1.0 {
        compact.pop();
    }
    if compact.contains(',') && compact.contains('.') {
        let comma = compact.rfind(',').unwrap_or_default();
        let dot = compact.rfind('.').unwrap_or_default();
        let normalized = if comma > dot {
            compact.replace('.', "").replace(',', ".")
        } else {
            compact.replace(',', "")
        };
        return crate::exchange::parse_decimal(&normalized, false)?
            .map(|value| value * multiplier)
            .ok_or_else(|| "Не прочитано число".into());
    }
    let separators: Vec<_> = compact.match_indices([',', '.']).collect();
    if separators.len() > 1 {
        return Err("Неоднозначные разделители — проверьте число".into());
    }
    if let Some((at, _)) = separators.first()
        && compact.len() - at - 1 == 3
        && multiplier == 1.0
    {
        return Err("Неясно, дробь это или разделитель тысяч".into());
    }
    crate::exchange::parse_decimal(&compact, true)?
        .map(|value| value * multiplier)
        .ok_or_else(|| "Не прочитано число".into())
}

fn read_stat(
    draft: &mut OcrDraft,
    values: &mut BTreeMap<String, f64>,
    conflicted: &mut std::collections::BTreeSet<String>,
    key: &str,
    raw: &str,
    confidence: f64,
    numbers: &Regex,
) {
    let raw_value = raw.split_once('(').map_or(raw, |(value, _)| value);
    let target = raw_value.rsplit_once('/').map_or(raw_value, |(_, max)| max);
    let number = numbers.find(target);
    let parsed = number
        .as_ref()
        .ok_or_else(|| "Число не прочитано".to_owned())
        .and_then(|n| ocr_number(n.as_str()));
    draft.evidence.push(OcrEvidence {
        field: key.into(),
        raw: raw.into(),
        value: parsed.as_ref().ok().copied(),
        confidence,
    });
    match parsed {
        Ok(value) => {
            if values.get(key).is_some_and(|old| *old != value) {
                conflicted.insert(key.into());
                draft.stats.set(key, None);
                draft
                    .warnings
                    .push(format!("Поле {key}: найдены разные значения"));
            } else if !conflicted.contains(key) {
                values.insert(key.into(), value);
                draft.stats.set(key, Some(value));
            }
            if let Some(number) = number
                && number
                    .as_str()
                    .chars()
                    .any(|c| matches!(c, 'k' | 'K' | 'к' | 'К' | 'm' | 'M' | 'м' | 'М'))
            {
                let label = STAT_FIELDS
                    .iter()
                    .find(|(field, _, _)| *field == key)
                    .map_or(key, |(_, label, _)| *label);
                draft.warnings.push(format!(
                    "{label}: {} ≈ {value}; исходное значение на скриншоте округлено",
                    number.as_str().trim()
                ));
            }
            if confidence < 65.0 {
                draft
                    .warnings
                    .push(format!("Поле {key}: проверьте качество распознавания"));
            }
        }
        Err(message) => draft.warnings.push(format!("Поле {key}: {message}")),
    }
}

pub fn parse_ocr(document: &OcrDocument, catalog: &[CreatureDefinition], game: Game) -> OcrDraft {
    let lines = grouped_lines(document);
    let mut draft = OcrDraft {
        raw_text: document.text.clone(),
        ..Default::default()
    };
    let numbers =
        Regex::new(r"-?\d(?:[\d., \x{00a0}]*\d)?\s*[kKmMкКмМ]?").expect("numeric pattern");
    let level = Regex::new(r"(?i)(?:level|lvl|уровень|ур\.?)[\s:.-]*(\d+)").expect("level pattern");
    let labels: &[(&str, &[&str])] = &[
        ("health", &["здоровье", "health"]),
        ("stamina", &["выносливость", "stamina"]),
        ("oxygen", &["кислород", "oxygen"]),
        ("food", &["пища", "еда", "food"]),
        ("weight", &["вес", "weight"]),
        (
            "melee_percent",
            &[
                "урон в ближнем бою",
                "урон от атак",
                "melee damage",
                "melee",
                "урон",
            ],
        ),
        (
            "speed_percent",
            &[
                "скорость передвижения",
                "movement speed",
                "скорость",
                "speed",
            ],
        ),
        ("torpor", &["оглушение", "torpidity", "torpor"]),
        (
            "imprint_percent",
            &["привязанность", "импринт", "imprinting", "imprint"],
        ),
    ];
    let mut values: BTreeMap<String, f64> = BTreeMap::new();
    let mut conflicted = std::collections::BTreeSet::new();
    for line in &lines {
        let text = line.text.trim();
        let lower = text.to_lowercase();
        if let Some(caps) = level.captures(text)
            && let Ok(n) = caps[1].parse::<u32>()
            && n > 0
        {
            if draft.level.is_some_and(|old| old != n) {
                draft
                    .warnings
                    .push("На скриншоте найдено несколько уровней — проверьте поле".into());
            } else {
                draft.level = Some(n);
            }
            if let Some(full) = caps.get(0) {
                let name = text[..full.start()].trim_matches([' ', '-', '—', ':', '·']);
                if !name.is_empty() && name.len() < 160 {
                    draft.nickname = Some(name.into());
                }
            }
        }
        if lower.contains("female") || lower.contains("самка") {
            draft.sex = Some("female".into());
        } else if lower.contains("male") || lower.contains("самец") {
            draft.sex = Some("male".into());
        }
        for (key, aliases) in labels {
            let alias = aliases.iter().find(|alias| {
                lower.starts_with(**alias)
                    && lower[alias.len()..]
                        .chars()
                        .next()
                        .is_none_or(|c| !c.is_alphabetic())
            });
            let Some(alias) = alias else {
                continue;
            };
            if lower.contains("points") || lower.contains("очк") {
                draft.warnings.push(format!(
                    "Поле {key}: очки статов требуют отдельного шаблона"
                ));
                continue;
            }
            let raw = text.get(alias.len()..).unwrap_or_default().trim();
            read_stat(
                &mut draft,
                &mut values,
                &mut conflicted,
                key,
                raw,
                line.confidence,
                &numbers,
            );
        }
    }
    if document.layout == "icon_panel" {
        let sex_line = document.lines.iter().find(|line| {
            matches!(
                line.text.trim().to_lowercase().as_str(),
                "male" | "female" | "самец" | "самка"
            )
        });
        if let Some(sex_box) = sex_line.and_then(|line| line.bbox) {
            draft.nickname = document
                .lines
                .iter()
                .filter(|line| {
                    line.bbox.is_some_and(|bounds| {
                        bounds[3] <= sex_box[1] + 2.0
                            && (bounds[0] - sex_box[0]).abs() < (sex_box[3] - sex_box[1])
                            && line.text.chars().any(char::is_alphabetic)
                    })
                })
                .max_by(|a, b| {
                    a.bbox.unwrap_or_default()[3].total_cmp(&b.bbox.unwrap_or_default()[3])
                })
                .map(|line| line.text.trim().to_owned());
            if let Some(header) = &draft.nickname
                && let Some(capture) = Regex::new(r"^(.+?)\s+(\d+)\s*$")
                    .expect("icon header pattern")
                    .captures(header)
                && let Ok(level) = capture[2].parse::<u32>()
                && level > 0
            {
                let name = capture[1].trim().to_owned();
                draft.level = Some(level);
                draft.nickname = Some(name);
            }
        }
        for (region, line) in &document.regions {
            let key = if region == "food_summary" {
                "food"
            } else {
                region.as_str()
            };
            if STAT_FIELDS.iter().any(|(field, _, _)| *field == key) {
                read_stat(
                    &mut draft,
                    &mut values,
                    &mut conflicted,
                    key,
                    &line.text,
                    line.confidence,
                    &numbers,
                );
            }
        }
        let tamed = Regex::new(r"(?i)(?:tamed|прируч[её]н(?:а|о)?)[\s:]*(?:\(\s*)?(\d+)")
            .expect("tamed level pattern");
        draft.tamed_level = tamed
            .captures(&document.text)
            .and_then(|capture| capture[1].parse::<u32>().ok())
            .filter(|level| *level > 0);
        let base_stage = if draft.tamed_level.is_some() {
            Some(PointStage::AfterTame)
        } else if Regex::new(r"(?i)(?:\bwild\b|дикий|дикая|дикое)")
            .expect("wild pattern")
            .is_match(&document.text)
        {
            Some(PointStage::BeforeTame)
        } else {
            None
        };
        let point_pattern =
            Regex::new(r"\(\s*(\d+)\s*[-–/]\s*(\d+)\s*[-–/]\s*(\d+)\s*\)").expect("point pattern");
        let mut point_conflicts = std::collections::BTreeSet::new();
        for (region, line) in &document.regions {
            let field = region.strip_prefix("points_").unwrap_or(region);
            let field = if field == "food_summary" {
                "food"
            } else {
                field
            };
            if !STAT_FIELDS
                .iter()
                .any(|(key, _, _)| *key == field && *key != "imprint_percent")
            {
                continue;
            }
            let Some(capture) = point_pattern.captures(&line.text) else {
                continue;
            };
            for (index, stage) in [
                (1, base_stage),
                (2, Some(PointStage::Mutations)),
                (3, Some(PointStage::Levelups)),
            ] {
                let Some(stage) = stage else {
                    continue;
                };
                let Ok(value) = capture[index].parse::<u32>() else {
                    continue;
                };
                let key = stage.field_key(field);
                if draft
                    .point_values
                    .get(&key)
                    .is_some_and(|old| *old != value)
                {
                    draft.point_values.remove(&key);
                    point_conflicts.insert(key.clone());
                    draft.warnings.push(format!(
                        "{field}: найдено несколько значений очков, проверьте вручную"
                    ));
                } else if !point_conflicts.contains(&key) {
                    draft.point_values.insert(key.clone(), value);
                }
                draft.evidence.push(OcrEvidence {
                    field: key,
                    raw: line.text.clone(),
                    value: Some(value as f64),
                    confidence: line.confidence,
                });
            }
        }
        draft.warnings.push("Очки из скобок перенесены отдельно от HP и процентов. База приручённого дино не восстанавливает его статы до приручения.".into());
    }
    if let Some(name) = draft.nickname.clone() {
        let naming = Regex::new(r"(?i)^\s*([mfмж])\s*\((\d+)\)\s*(\d+)\s*\.\s*(\d+)\s*\.\s*(\d+)\s*\.\s*(\d+)\s*\.\s*(\d+)\s*\.\s*(\d+)(?:\.|\s|$)").expect("breeding name pattern");
        if let Some(capture) = naming.captures(&name) {
            let keys = [
                "health",
                "stamina",
                "food",
                "melee_percent",
                "oxygen",
                "weight",
            ];
            let points: Option<Vec<u32>> = (3..9)
                .map(|index| capture[index].parse::<u32>().ok())
                .collect();
            if let Some(points) = points {
                for (key, value) in keys.into_iter().zip(points) {
                    let field = PointStage::BeforeTame.field_key(key);
                    if draft
                        .point_values
                        .get(&field)
                        .is_some_and(|old| *old != value)
                    {
                        draft.point_values.remove(&field);
                        draft.warnings.push(format!("{key}: очки из имени отличаются от диких очков на панели; подтвердите вручную"));
                    } else {
                        draft.point_values.insert(field, value);
                    }
                }
                draft.warnings.push("Очки из метки имени перенесены в снимок «До приручения»: HP / выносливость / еда / урон / кислород / вес. Проверьте шаблон имени и происхождение этих значений.".into());
            }
        }
    }
    let searchable = format!(
        "{}\n{}",
        document.text,
        lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    )
    .to_lowercase();
    for c in catalog.iter().filter(|c| c.game == game) {
        let found = std::iter::once(&c.name)
            .chain(std::iter::once(&c.name_en))
            .chain(c.aliases.iter())
            .any(|name| {
                let pattern = format!(
                    r"(?i)(?:^|[^\p{{L}}\p{{N}}]){}(?:$|[^\p{{L}}\p{{N}}])",
                    regex::escape(name)
                );
                Regex::new(&pattern).is_ok_and(|r| r.is_match(&searchable))
            });
        if found {
            draft.candidates.push(c.id.clone());
        }
    }
    if draft.candidates.is_empty()
        && let Some(family) = &document.portrait_family
    {
        let compact = |text: &str| {
            text.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
                .to_lowercase()
        };
        let family_key = compact(family);
        draft.candidates = catalog
            .iter()
            .filter(|creature| {
                creature.game == game
                    && creature.kind != "boss"
                    && compact(&creature.name_en).contains(&family_key)
            })
            .map(|creature| creature.id.clone())
            .collect();
        draft.portrait_family = Some(family.clone());
        draft.warnings.push(format!("По портрету определено семейство {family}. Разновидность и модовый вариант подтвердите в списке."));
    }
    if draft.candidates.len() != 1 {
        draft
            .warnings
            .push("Подтвердите вид и вариант существа в каталоге".into());
    }
    if draft.evidence.is_empty() {
        draft.warnings.push(
            "Статы не найдены. Выделите окно характеристик целиком или заполните поля вручную."
                .into(),
        );
    }
    draft.warnings.sort();
    draft.warnings.dedup();
    draft
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(text: &str) -> OcrDraft {
        parse_ocr(
            &OcrDocument {
                text: text.into(),
                lines: vec![],
                ..Default::default()
            },
            &[],
            Game::Asa,
        )
    }
    #[test]
    fn english_values_use_maximum_and_keep_percent_units() {
        let d = parse(
            "Boris - Lvl 224\nHealth: 3210.5 / 5000.0\nStamina: 400 / 1200\nMelee Damage: 350.5%\nFemale",
        );
        assert_eq!(d.level, Some(224));
        assert_eq!(d.nickname.as_deref(), Some("Boris"));
        assert_eq!(d.stats.health, Some(5000.0));
        assert_eq!(d.stats.stamina, Some(1200.0));
        assert_eq!(d.stats.melee_percent, Some(350.5));
        assert_eq!(d.sex.as_deref(), Some("female"));
    }
    #[test]
    fn russian_decimal_comma_and_missing_values() {
        let d = parse(
            "Борис — Ур. 123\nЗдоровье 0 / 4500,5\nВес 12,5 / 800,0\nПривязанность 100%\nКислород ???",
        );
        assert_eq!(d.stats.health, Some(4500.5));
        assert_eq!(d.stats.weight, Some(800.0));
        assert_eq!(d.stats.imprint_percent, Some(100.0));
        assert_eq!(d.stats.oxygen, None);
    }
    #[test]
    fn ambiguous_number_is_not_guessed() {
        assert_eq!(parse("Health 4,500").stats.health, None);
        assert_eq!(parse("Health 1,500.5").stats.health, Some(1500.5));
    }
    #[test]
    fn conflicting_stat_is_not_silently_selected() {
        let d = parse("Health 3000\nHealth 5000");
        assert_eq!(d.stats.health, None);
    }
    #[test]
    fn unrelated_numbers_do_not_shift_into_missing_fields() {
        let d = parse("Armor 25\nXP 1234\nHealth 5000\nStamina ???\nWeight 200");
        assert_eq!(d.stats.health, Some(5000.0));
        assert_eq!(d.stats.stamina, None);
        assert_eq!(d.stats.weight, Some(200.0));
    }
    #[test]
    fn matching_vertical_boxes_join_label_and_value() {
        let d = parse_ocr(
            &OcrDocument {
                text: "Health\n3000 / 5000".into(),
                lines: vec![
                    OcrLine {
                        text: "3000 / 5000".into(),
                        confidence: 98.0,
                        bbox: Some([200.0, 10.0, 300.0, 30.0]),
                    },
                    OcrLine {
                        text: "Health".into(),
                        confidence: 99.0,
                        bbox: Some([10.0, 11.0, 100.0, 31.0]),
                    },
                ],
                ..Default::default()
            },
            &[],
            Game::Asa,
        );
        assert_eq!(d.stats.health, Some(5000.0));
    }
    #[test]
    fn wild_points_require_another_template() {
        assert_eq!(parse("Health wild points 45").stats.health, None);
    }

    fn icon_panel(regions: &[(&str, &str)]) -> OcrDocument {
        OcrDocument {
            text: "Male\nTamed (211)\nReborn 280".into(),
            layout: "icon_panel".into(),
            regions: regions
                .iter()
                .map(|(key, text)| {
                    (
                        (*key).into(),
                        OcrLine {
                            text: (*text).into(),
                            confidence: 96.0,
                            bbox: None,
                        },
                    )
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn icon_panel_reads_maxima_and_suffixes_without_using_point_counts() {
        let document = icon_panel(&[
            ("health", "6.8K / 8.1K (29-0-10)"),
            ("food", "8.4K / 8.5K (37-0-0)"),
            ("torpor", "0.0/9.9К (0-0-0)"),
            ("stamina", "1.4K"),
            ("oxygen", "525.0"),
            ("food_summary", "8.5K"),
            ("weight", "1.6K"),
            ("melee_percent", "700.6%"),
            ("speed_percent", "100.0%"),
        ]);
        let d = parse_ocr(&document, &[], Game::Asa);
        assert_eq!(d.stats.health, Some(8100.0));
        assert_eq!(d.stats.food, Some(8500.0));
        assert_eq!(d.stats.torpor, Some(9900.0));
        assert_eq!(d.stats.stamina, Some(1400.0));
        assert_eq!(d.stats.oxygen, Some(525.0));
        assert_eq!(d.stats.weight, Some(1600.0));
        assert_eq!(d.stats.melee_percent, Some(700.6));
        assert_eq!(d.stats.speed_percent, Some(100.0));
        assert_eq!(d.stats.imprint_percent, None);
        assert_eq!(d.sex.as_deref(), Some("male"));
        assert_eq!(d.tamed_level, Some(211));
        assert_eq!(d.level, None);
        assert!(
            d.warnings
                .iter()
                .any(|warning| warning.contains("8.1K ≈ 8100"))
        );
    }

    #[test]
    fn missing_icon_value_does_not_shift_other_stats() {
        let d = parse_ocr(
            &icon_panel(&[("stamina", ""), ("oxygen", "525.0"), ("weight", "1.6K")]),
            &[],
            Game::Asa,
        );
        assert_eq!(d.stats.stamina, None);
        assert_eq!(d.stats.oxygen, Some(525.0));
        assert_eq!(d.stats.weight, Some(1600.0));
        assert_eq!(d.stats.health, None);
    }

    #[test]
    fn conflicting_panel_values_require_manual_confirmation() {
        let d = parse_ocr(
            &icon_panel(&[("food", "8.4K / 8.5K (37-0-0)"), ("food_summary", "8.6K")]),
            &[],
            Game::Asa,
        );
        assert_eq!(d.stats.food, None);
    }

    #[test]
    fn labeled_stats_also_accept_abbreviated_numbers() {
        let d = parse("Health 1,2М\nStamina 1.4K\nWeight 1.234K\nFood 8.4K / 8.5K (37-0-0)");
        assert_eq!(d.stats.health, Some(1_200_000.0));
        assert_eq!(d.stats.stamina, Some(1400.0));
        assert_eq!(d.stats.weight, Some(1234.0));
        assert_eq!(d.stats.food, Some(8500.0));
    }

    #[test]
    fn header_name_and_pre_tame_points_remain_separate_from_tamed_snapshot() {
        let mut document = icon_panel(&[
            ("health", "6.8K / 8.1K (35-2-10)"),
            ("points_stamina", "(48-0-10)"),
        ]);
        document.lines = vec![
            OcrLine {
                text: "M(211)29.40.37.50.25.29. Reborn 280".into(),
                bbox: Some([180.0, 50.0, 540.0, 75.0]),
                confidence: 96.0,
            },
            OcrLine {
                text: "Male".into(),
                bbox: Some([180.0, 90.0, 250.0, 115.0]),
                confidence: 96.0,
            },
            OcrLine {
                text: "Owner".into(),
                bbox: Some([180.0, 125.0, 300.0, 150.0]),
                confidence: 96.0,
            },
        ];
        let d = parse_ocr(&document, &[], Game::Asa);
        assert_eq!(
            d.nickname.as_deref(),
            Some("M(211)29.40.37.50.25.29. Reborn")
        );
        assert_eq!(d.level, Some(280));
        assert_eq!(d.tamed_level, Some(211));
        assert_eq!(d.point_values.get("points.before_tame.health"), Some(&29));
        assert_eq!(d.point_values.get("points.after_tame.health"), Some(&35));
        assert_eq!(d.point_values.get("points.before_tame.stamina"), Some(&40));
        assert_eq!(d.point_values.get("points.after_tame.stamina"), Some(&48));
        assert_eq!(d.point_values.get("points.before_tame.food"), Some(&37));
        assert_eq!(
            d.point_values.get("points.before_tame.melee_percent"),
            Some(&50)
        );
        assert_eq!(d.point_values.get("points.before_tame.oxygen"), Some(&25));
        assert_eq!(d.point_values.get("points.before_tame.weight"), Some(&29));
        assert_eq!(d.point_values.get("points.mutations.health"), Some(&2));
        assert_eq!(d.point_values.get("points.levelups.health"), Some(&10));
    }

    #[test]
    fn tamed_snapshot_does_not_invent_pre_tame_points() {
        let d = parse_ocr(
            &icon_panel(&[("health", "6.8K / 8.1K (29-0-10)")]),
            &[],
            Game::Asa,
        );
        assert_eq!(d.point_values.get("points.after_tame.health"), Some(&29));
        assert!(!d.point_values.contains_key("points.before_tame.health"));
    }
}
