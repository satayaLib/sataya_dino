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
    pub sex: Option<String>,
    pub stats: Stats,
    pub candidates: Vec<String>,
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
    let compact = text.replace([' ', '\u{a0}'], "");
    if compact.contains(',') && compact.contains('.') {
        let comma = compact.rfind(',').unwrap_or_default();
        let dot = compact.rfind('.').unwrap_or_default();
        let normalized = if comma > dot {
            compact.replace('.', "").replace(',', ".")
        } else {
            compact.replace(',', "")
        };
        return crate::exchange::parse_decimal(&normalized, false)?
            .ok_or_else(|| "Не прочитано число".into());
    }
    let separators: Vec<_> = compact.match_indices([',', '.']).collect();
    if separators.len() > 1 {
        return Err("Неоднозначные разделители — проверьте число".into());
    }
    if let Some((at, _)) = separators.first()
        && compact.len() - at - 1 == 3
    {
        return Err("Неясно, дробь это или разделитель тысяч".into());
    }
    crate::exchange::parse_decimal(&compact, true)?.ok_or_else(|| "Не прочитано число".into())
}

pub fn parse_ocr(document: &OcrDocument, catalog: &[CreatureDefinition], game: Game) -> OcrDraft {
    let lines = grouped_lines(document);
    let mut draft = OcrDraft {
        raw_text: document.text.clone(),
        ..Default::default()
    };
    let numbers = Regex::new(r"-?\d[\d., \x{00a0}]*").expect("numeric pattern");
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
            let target = raw.rsplit_once('/').map_or(raw, |(_, max)| max);
            let parsed = numbers
                .find(target)
                .ok_or_else(|| "Число не прочитано".to_owned())
                .and_then(|n| ocr_number(n.as_str()));
            let value = parsed.as_ref().ok().copied();
            draft.evidence.push(OcrEvidence {
                field: (*key).into(),
                raw: text.into(),
                value,
                confidence: line.confidence,
            });
            match parsed {
                Ok(value) => {
                    if values.get(*key).is_some_and(|old| *old != value) {
                        conflicted.insert((*key).to_owned());
                        draft.stats.set(key, None);
                        draft
                            .warnings
                            .push(format!("Поле {key}: найдены разные значения"));
                    } else if !conflicted.contains(*key) {
                        values.insert((*key).into(), value);
                        draft.stats.set(key, Some(value));
                    }
                    if line.confidence < 65.0 {
                        draft
                            .warnings
                            .push(format!("Поле {key}: проверьте качество распознавания"));
                    }
                }
                Err(message) => draft.warnings.push(format!("Поле {key}: {message}")),
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
    if draft.candidates.len() != 1 {
        draft
            .warnings
            .push("Подтвердите вид и вариант существа в каталоге".into());
    }
    if draft.evidence.is_empty() {
        draft.warnings.push(
            "Статы с подписями не найдены. Измените область или заполните поля вручную.".into(),
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
}
