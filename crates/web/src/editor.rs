use crate::{browser, ocr_view::OcrPanel, state::*, ui::*};
use ark_core::*;
use std::collections::{BTreeMap, BTreeSet};
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement, SubmitEvent};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct EditorProps {
    pub target: EditorTarget,
}

fn initial(ctx: &AppContext, target: &EditorTarget) -> OwnedCreature {
    let mut profile = ctx.data.profile();
    match target {
        EditorTarget::Edit(id) | EditorTarget::Copy(id) => {
            if let Some(c) = ctx.data.creatures.iter().find(|c| &c.id == id) {
                let mut c = c.clone();
                if matches!(target, EditorTarget::Copy(_)) {
                    c.id = browser::new_id();
                    c.nickname = format!("{} (копия)", c.title());
                    c.created_at = browser::now();
                    c.updated_at = c.created_at.clone();
                }
                return c;
            }
        }
        _ => (),
    }
    let selected = if let EditorTarget::New(Some(id), _) = target {
        ctx.catalog.creatures.iter().find(|c| &c.id == id)
    } else {
        None
    };
    if let Some(c) = selected
        && let Some(p) = ctx.data.profiles.iter().find(|p| p.game == c.game)
    {
        profile = p;
    }
    let reference = selected.map(|c| c.reference()).unwrap_or(CreatureRef {
        creature_id: String::new(),
        game: profile.game,
        pack_id: String::new(),
        pack_version: String::new(),
        catalog_revision: String::new(),
        creature_name: String::new(),
    });
    let mut c = OwnedCreature::new(
        browser::new_id(),
        profile.id.clone(),
        reference,
        browser::now(),
    );
    c.map_id = profile.map.clone();
    c
}

#[component]
pub fn Editor(props: &EditorProps) -> Html {
    let ctx = use_app();
    let draft = use_state(|| initial(&ctx, &props.target));
    let original = use_state(|| (*draft).clone());
    let values = use_state(|| {
        STAT_FIELDS
            .iter()
            .map(|(k, _, _)| {
                (
                    k.to_string(),
                    draft
                        .stats
                        .get(k)
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                )
            })
            .collect::<BTreeMap<_, _>>()
    });
    let level = use_state(|| draft.level.map(|v| v.to_string()).unwrap_or_default());
    let tamed = use_state(|| draft.tamed_level.map(|v| v.to_string()).unwrap_or_default());
    let tags = use_state(|| draft.tags.join(", "));
    let mode = use_state(|| {
        if matches!(props.target, EditorTarget::New(_, true)) {
            "screenshot".to_owned()
        } else {
            "manual".to_owned()
        }
    });
    let edited = use_state(BTreeSet::<String>::new);
    let point_values = use_state(|| {
        POINT_STAGES
            .iter()
            .flat_map(|(stage, _)| {
                STAT_FIELDS
                    .iter()
                    .filter(|(key, _, _)| *key != "imprint_percent")
                    .map(|(key, _, _)| {
                        (
                            stage.field_key(key),
                            draft
                                .point(*stage, key)
                                .map(|value| value.to_string())
                                .unwrap_or_default(),
                        )
                    })
            })
            .collect::<BTreeMap<_, _>>()
    });
    let errors = use_state(Vec::<FieldError>::new);
    let evidence = use_state(|| None::<OcrDraft>);
    let overwrite = use_state(|| false);
    let close = {
        let ctx = ctx.clone();
        Callback::from(move |_| ctx.editor.set(None))
    };
    let onkind = {
        let draft = draft.clone();
        let catalog = ctx.catalog.clone();
        let edited = edited.clone();
        Callback::from(move |e: Event| {
            let id = e.target_unchecked_into::<HtmlSelectElement>().value();
            if let Some(c) = catalog.creatures.iter().find(|c| c.id == id) {
                let mut d = (*draft).clone();
                d.reference = c.reference();
                draft.set(d);
                let mut v = (*edited).clone();
                v.insert("creature_id".into());
                edited.set(v);
            }
        })
    };
    let onprofile = {
        let draft = draft.clone();
        let profiles = ctx.data.profiles.clone();
        Callback::from(move |e: Event| {
            let id = e.target_unchecked_into::<HtmlSelectElement>().value();
            if let Some(p) = profiles.iter().find(|p| p.id == id) {
                let mut d = (*draft).clone();
                d.profile_id = p.id.clone();
                if d.reference.game != p.game {
                    d.reference = CreatureRef {
                        game: p.game,
                        creature_id: String::new(),
                        pack_id: String::new(),
                        pack_version: String::new(),
                        catalog_revision: String::new(),
                        creature_name: String::new(),
                    };
                }
                draft.set(d);
            }
        })
    };
    let onname = {
        let draft = draft.clone();
        let edited = edited.clone();
        Callback::from(move |e: InputEvent| {
            let mut d = (*draft).clone();
            d.nickname = e.target_unchecked_into::<HtmlInputElement>().value();
            draft.set(d);
            let mut v = (*edited).clone();
            v.insert("nickname".into());
            edited.set(v);
        })
    };
    let onlevel = {
        let level = level.clone();
        let edited = edited.clone();
        Callback::from(move |e: InputEvent| {
            level.set(e.target_unchecked_into::<HtmlInputElement>().value());
            let mut v = (*edited).clone();
            v.insert("level".into());
            edited.set(v);
        })
    };
    let onocr = {
        let draft = draft.clone();
        let values = values.clone();
        let level = level.clone();
        let tamed = tamed.clone();
        let edited = edited.clone();
        let evidence = evidence.clone();
        let overwrite = overwrite.clone();
        let point_values = point_values.clone();
        let catalog = ctx.catalog.clone();
        Callback::from(move |ocr: OcrDraft| {
            let mut d = (*draft).clone();
            let may = |key: &str| *overwrite || !edited.contains(key);
            if may("nickname")
                && let Some(v) = &ocr.nickname
            {
                d.nickname = v.clone();
            }
            if may("level")
                && let Some(v) = ocr.level
            {
                level.set(v.to_string());
            }
            if may("tamed_level")
                && let Some(v) = ocr.tamed_level
            {
                tamed.set(v.to_string());
            }
            if may("sex")
                && let Some(v) = &ocr.sex
            {
                d.sex = v.clone();
            }
            if d.reference.creature_id.is_empty()
                && ocr.candidates.len() == 1
                && let Some(c) = catalog.creatures.iter().find(|c| c.id == ocr.candidates[0])
            {
                d.reference = c.reference();
            }
            let mut fields = (*values).clone();
            for (key, _, _) in STAT_FIELDS {
                if may(key)
                    && let Some(v) = ocr.stats.get(key)
                {
                    fields.insert(key.into(), v.to_string());
                }
            }
            values.set(fields);
            let mut points = (*point_values).clone();
            for (key, value) in &ocr.point_values {
                let saved_before = key.starts_with("points.before_tame.")
                    && points
                        .get(key)
                        .is_some_and(|value| !value.trim().is_empty());
                if may(key) && (!saved_before || *overwrite) {
                    points.insert(key.clone(), value.to_string());
                }
            }
            point_values.set(points);
            draft.set(d);
            evidence.set(Some(ocr));
        })
    };
    let submit = {
        let ctx = ctx.clone();
        let draft = draft.clone();
        let original = original.clone();
        let values = values.clone();
        let level = level.clone();
        let tamed = tamed.clone();
        let tags = tags.clone();
        let errors = errors.clone();
        let target = props.target.clone();
        let point_values = point_values.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let mut d = (*draft).clone();
            let mut issues = vec![];
            for (stage, _) in POINT_STAGES {
                for (key, _, _) in STAT_FIELDS
                    .iter()
                    .filter(|(key, _, _)| *key != "imprint_percent")
                {
                    let field = stage.field_key(key);
                    let raw = point_values
                        .get(&field)
                        .map(String::as_str)
                        .unwrap_or("")
                        .trim();
                    if raw.is_empty() {
                        d.set_point(stage, key, None);
                    } else {
                        match raw.parse::<u32>() {
                            Ok(value) => d.set_point(stage, key, Some(value)),
                            Err(_) => issues
                                .push(FieldError::new(&field, "Нужно целое неотрицательное число")),
                        }
                    }
                }
            }
            for (key, _, _) in STAT_FIELDS {
                let raw = values.get(key).map(String::as_str).unwrap_or("");
                match parse_decimal(raw, raw.contains(',')) {
                    Ok(v) => d.stats.set(key, v),
                    Err(message) => issues.push(FieldError {
                        field: key.into(),
                        message,
                    }),
                }
            }
            for (key, raw) in [("level", level.as_str()), ("tamed_level", tamed.as_str())] {
                let parsed = if raw.trim().is_empty() {
                    None
                } else {
                    match raw.trim().parse::<u32>() {
                        Ok(n) if n > 0 => Some(n),
                        _ => {
                            issues.push(FieldError::new(key, "Нужно положительное целое"));
                            None
                        }
                    }
                };
                if key == "level" {
                    d.level = parsed;
                } else {
                    d.tamed_level = parsed;
                }
            }
            d.tags = tags
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect();
            d.updated_at = browser::now();
            issues.extend(d.validate());
            if matches!(target, EditorTarget::Edit(_))
                && ctx.data.creatures.iter().find(|c| c.id == original.id) != Some(&*original)
            {
                issues.push(FieldError::new("id","Эта запись изменилась в другой вкладке. Скопируйте свои правки и заново откройте актуальную запись."));
            }
            if !issues.is_empty() {
                errors.set(issues);
                return;
            }
            let mut next = (*ctx.data).clone();
            if let Some(c) = next.creatures.iter_mut().find(|c| c.id == d.id) {
                *c = d;
            } else {
                next.creatures.push(d);
            }
            let editor = ctx.editor.clone();
            ctx.save(
                next,
                "Дино сохранён",
                Callback::from(move |_| editor.set(None)),
            );
        })
    };
    let change_select = |key: &'static str| {
        let draft = draft.clone();
        let edited = edited.clone();
        Callback::from(move |e: Event| {
            let v = e.target_unchecked_into::<HtmlSelectElement>().value();
            let mut d = (*draft).clone();
            match key {
                "sex" => d.sex = v,
                "status" => d.status = v,
                _ => d.stat_stage = v,
            }
            draft.set(d);
            let mut v = (*edited).clone();
            v.insert(key.into());
            edited.set(v);
        })
    };
    let title = if matches!(props.target, EditorTarget::Edit(_)) {
        "Редактирование дино"
    } else {
        "Новый дино в стае"
    };
    let unknown = !draft.reference.creature_id.is_empty()
        && !ctx
            .catalog
            .creatures
            .iter()
            .any(|c| c.id == draft.reference.creature_id);
    html! {<Modal title={title} onclose={close.clone()} wide={true}><form onsubmit={submit}><div class="editor-tabs">{for[("manual","Вручную","plus"),("screenshot","По скриншоту","photo")].iter().map(|(key,label,icon)|{let m=mode.clone();let key=key.to_string();html!{<button type="button" class={classes!("tab",(*mode==key).then_some("active"))} onclick={Callback::from(move |_|m.set(key.clone()))}><Icon name={*icon}/>{label}</button>}})}</div><div class={classes!("editor-body",(*mode=="screenshot").then_some("with-ocr"))}>
    if *mode=="screenshot"{<OcrPanel game={draft.reference.game} onrecognized={onocr}/>}<div class="editor-fields">
    <div class="form-grid"><label>{"Мир / сервер"}<SelectField value={draft.profile_id.clone()} onchange={onprofile}>{for ctx.data.profiles.iter().map(|p|html!{<option value={p.id.clone()}>{&p.name}</option>})}</SelectField></label><label>{"Вид и вариант"}<SelectField label="Вид и вариант дино" value={draft.reference.creature_id.clone()} onchange={onkind} required={true}><option value="">{"Выберите из каталога"}</option>if unknown{<option value={draft.reference.creature_id.clone()}>{format!("{} · пакет недоступен",draft.reference.creature_name)}</option>}{for ctx.catalog.creatures.iter().filter(|c|c.game==draft.reference.game).map(|c|html!{<option value={c.id.clone()}>{format!("{} · {}",c.name,if c.pack_id=="vanilla"{"база"}else{c.pack_id.as_str()})}</option>})}</SelectField></label><label class="span-2">{"Имя дино"}<input placeholder="Например, Борис" value={draft.nickname.clone()} oninput={onname} maxlength="200"/></label><label>{"Пол"}<SelectField value={draft.sex.clone()} onchange={change_select("sex")}><option value="unknown">{"Не указан"}</option><option value="male">{"Самец"}</option><option value="female">{"Самка"}</option></SelectField></label><label>{"Текущий уровень"}<input inputmode="numeric" placeholder="Неизвестно" value={(*level).clone()} oninput={onlevel}/></label></div>
    if let Some(ocr)=&*evidence && !ocr.candidates.is_empty(){<div class="species-candidates"><strong>{if ocr.portrait_family.is_some(){"Варианты по портрету — подтвердите своего дино"}else{"Возможные виды — подтвердите вариант"}}</strong><div class="button-row">{for ctx.catalog.creatures.iter().filter(|creature|creature.game==draft.reference.game&&ocr.candidates.contains(&creature.id)).map(|creature|{let choose={let draft=draft.clone();let edited=edited.clone();let reference=creature.reference();Callback::from(move |_|{let mut value=(*draft).clone();value.reference=reference.clone();draft.set(value);let mut fields=(*edited).clone();fields.insert("creature_id".into());edited.set(fields);})};html!{<button type="button" class={classes!("button","small-button",if draft.reference.creature_id==creature.id{"primary"}else{"secondary"})} onclick={choose}>{&creature.name}</button>}})}</div></div>}<div class="section-heading stats-heading"><h3>{"Характеристики"}</h3><span class="small muted">{"Максимальные значения из игры"}</span></div><div class="stats-grid">{for STAT_FIELDS.iter().map(|(key,label,unit)|{
        let value=values.get(*key).cloned().unwrap_or_default();let suspect=evidence.as_ref().is_some_and(|e|e.evidence.iter().any(|v|v.field==*key&&(v.confidence<65.0||v.value.is_none())));
        let update={let values=values.clone();let edited=edited.clone();let key=key.to_string();Callback::from(move |e:InputEvent|{let mut v=(*values).clone();v.insert(key.clone(),e.target_unchecked_into::<HtmlInputElement>().value());values.set(v);let mut v=(*edited).clone();v.insert(key.clone());edited.set(v);})};
        html!{<label class={classes!(suspect.then_some("review-field"))}>{label}<div class="unit-input"><input aria-label={format!("Стат: {label}")} inputmode="decimal" placeholder="—" value={value} oninput={update}/><span>{unit}</span></div></label>}
    })}</div>
    <section class="points-section"><div class="section-heading stats-heading"><h3>{"Очки характеристик"}</h3><span class="small muted">{"Отдельно от HP и процентов"}</span></div><p class="muted small">{"Два снимка хранятся отдельно. Повторное распознавание сохраняет заполненный снимок до приручения, пока не разрешена его замена. Пустое поле означает, что очки неизвестны."}</p><div class="table-wrap"><table class="points-table"><thead><tr><th>{"Характеристика"}</th>{for POINT_STAGES.iter().map(|(_, label)|html!{<th>{label}</th>})}</tr></thead><tbody>{for STAT_FIELDS.iter().filter(|(key, _, _)| *key != "imprint_percent").map(|(key, label, _)|html!{<tr><th>{label}</th>{for POINT_STAGES.iter().map(|(stage, stage_label)|{
        let field=stage.field_key(key);let value=point_values.get(&field).cloned().unwrap_or_default();let update={let point_values=point_values.clone();let edited=edited.clone();let field=field.clone();Callback::from(move |event:InputEvent|{let mut points=(*point_values).clone();points.insert(field.clone(),event.target_unchecked_into::<HtmlInputElement>().value());point_values.set(points);let mut touched=(*edited).clone();touched.insert(field.clone());edited.set(touched);})};
        html!{<td><input aria-label={format!("Очки: {label} · {stage_label}")} inputmode="numeric" placeholder="—" value={value} oninput={update}/></td>}
    })}</tr>})}</tbody></table></div></section>
    if *mode=="screenshot"{<label class="checkbox-label"><input type="checkbox" checked={*overwrite} onchange={let v=overwrite.clone();Callback::from(move |e:Event|v.set(e.target_unchecked_into::<HtmlInputElement>().checked()))}/>{"При повторном распознавании разрешить замену снимка до приручения и ручных исправлений"}</label>}
    <div class="form-grid"><label>{"Статы измерены"}<SelectField value={draft.stat_stage.clone()} onchange={change_select("stat_stage")}><option value="current">{"После прокачки"}</option><option value="after_tame">{"После приручения"}</option><option value="newborn">{"При рождении"}</option></SelectField></label><label>{"Статус"}<SelectField value={draft.status.clone()} onchange={change_select("status")}>{for["active","cryopod","dead","transferred"].map(|s|html!{<option value={s}>{status_label(s)}</option>})}</SelectField></label><label>{"Уровень после приручения"}<input inputmode="numeric" value={(*tamed).clone()} placeholder="Если известен" oninput={let tamed=tamed.clone();let edited=edited.clone();Callback::from(move |e:InputEvent|{tamed.set(e.target_unchecked_into::<HtmlInputElement>().value());let mut fields=(*edited).clone();fields.insert("tamed_level".into());edited.set(fields);})}/></label><label>{"Карта"}<input value={draft.map_id.clone()} oninput={let draft=draft.clone();Callback::from(move |e:InputEvent|{let mut d=(*draft).clone();d.map_id=e.target_unchecked_into::<HtmlInputElement>().value();draft.set(d);})}/></label><label class="span-2">{"Теги через запятую"}<input value={(*tags).clone()} placeholder="Для боссов, на разведение…" oninput={let tags=tags.clone();Callback::from(move |e:InputEvent|tags.set(e.target_unchecked_into::<HtmlInputElement>().value()))}/></label><label class="span-2">{"Полевые заметки"}<textarea rows="3" value={draft.notes.clone()} placeholder="Где живёт, чем полезен, что важно помнить…" oninput={let draft=draft.clone();Callback::from(move |e:InputEvent|{let mut d=(*draft).clone();d.notes=e.target_unchecked_into::<HtmlTextAreaElement>().value();draft.set(d);})}/></label></div>
    if !errors.is_empty(){<div class="error-box" role="alert">{for errors.iter().map(|e|html!{<p>{format!("{}: {}",e.field,e.message)}</p>})}</div>}
    if let Some(error)=&*ctx.storage_error{<div class="error-box">{error}</div>}
    </div></div><footer class="modal-footer"><span class="muted small">{if *ctx.busy{"Сохраняем в браузере…"}else{"Пустое поле = значение неизвестно"}}</span><div class="button-row"><button type="button" class="button secondary" onclick={let close=close.clone();Callback::from(move |_|close.emit(()))}>{"Отмена"}</button><button type="submit" class="button primary" disabled={*ctx.busy||!*ctx.ready}><Icon name="check"/>{if *mode=="screenshot"{"Подтвердить и сохранить"}else{"Сохранить дино"}}</button></div></footer></form></Modal>}
}
