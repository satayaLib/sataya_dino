use crate::{browser, state::*, ui::*};
use ark_core::*;
use std::collections::BTreeMap;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;

#[component]
pub fn TransferView() -> Html {
    let ctx = use_app();
    let text = use_state(String::new);
    let filename = use_state(String::new);
    let fileerror = use_state(|| None::<String>);
    let loading = use_state(|| false);
    let delimiter = use_state(|| "auto".to_owned());
    let decimal = use_state(|| false);
    let mapping = use_state(BTreeMap::<String, String>::new);
    let mode = use_state(|| "add".to_owned());
    let skip = use_state(|| false);
    let confirm = use_state(|| false);
    let overrides = use_state(BTreeMap::<usize, String>::new);
    let is_json = filename.to_lowercase().ends_with(".json") || text.trim_start().starts_with('{');
    let onfile = {
        let text = text.clone();
        let filename = filename.clone();
        let fileerror = fileerror.clone();
        let loading = loading.clone();
        let mapping = mapping.clone();
        let overrides = overrides.clone();
        let confirm = confirm.clone();
        let mode = mode.clone();
        Callback::from(move |e: Event| {
            let input = e.target_unchecked_into::<HtmlInputElement>();
            let Some(file) = input.files().and_then(|f| f.get(0)) else {
                return;
            };
            input.set_value("");
            let text = text.clone();
            let filename = filename.clone();
            let fileerror = fileerror.clone();
            let loading = loading.clone();
            let mode = mode.clone();
            loading.set(true);
            fileerror.set(None);
            text.set(String::new());
            mapping.set(BTreeMap::new());
            overrides.set(BTreeMap::new());
            confirm.set(false);
            spawn_local(async move {
                match browser::read_text(file.clone().into()).await {
                    Ok(value) => {
                        mode.set(
                            if file.name().to_lowercase().ends_with(".json")
                                || value.trim_start().starts_with('{')
                            {
                                "merge".into()
                            } else {
                                "add".into()
                            },
                        );
                        filename.set(file.name());
                        text.set(value);
                    }
                    Err(e) => fileerror.set(Some(browser::error(e))),
                }
                loading.set(false);
            });
        })
    };
    let guessed = if text
        .lines()
        .next()
        .is_some_and(|s| s.matches(';').count() > s.matches(',').count())
    {
        b';'
    } else {
        b','
    };
    let delimiter_value = match delimiter.as_str() {
        "comma" => b',',
        "semicolon" => b';',
        _ => guessed,
    };
    let headers = if !text.is_empty() && !is_json {
        csv_headers(&text, delimiter_value).unwrap_or_default()
    } else {
        vec![]
    };
    let deps = (
        (*text).clone(),
        delimiter_value,
        *decimal,
        (*mapping).clone(),
        (*overrides).clone(),
        ctx.data.revision,
        is_json,
        ctx.data.profile().id.clone(),
    );
    let preview = {
        let ctx = ctx.clone();
        use_memo(
            deps,
            move |(text, delimiter, decimal, mapping, overrides, _, is_json, _)| {
                if text.is_empty() || *is_json {
                    return None;
                }
                let options = CsvOptions {
                    delimiter: Some(*delimiter),
                    decimal_comma: *decimal,
                    mapping: mapping.clone(),
                    default_profile: Some(ctx.data.profile().clone()),
                    default_creature: None,
                };
                let mut result = preview_csv(
                    text,
                    &ctx.data,
                    &ctx.catalog.creatures,
                    &options,
                    &browser::now(),
                    browser::new_id,
                );
                if let Ok(p) = &mut result {
                    for row in &mut p.rows {
                        if let Some(id) = overrides.get(&row.line)
                            && let Some(c) = ctx
                                .catalog
                                .creatures
                                .iter()
                                .find(|c| &c.id == id && c.game == row.value.reference.game)
                        {
                            row.value.reference = c.reference();
                            row.errors.retain(|e| {
                                !["creature_id", "creature_name", "pack_id"]
                                    .contains(&e.field.as_str())
                            });
                            row.provided.extend([
                                "creature_id".into(),
                                "creature_name".into(),
                                "pack_id".into(),
                                "pack_version".into(),
                                "catalog_revision".into(),
                            ]);
                        }
                    }
                }
                Some(result)
            },
        )
    };
    let backup = if is_json && !text.is_empty() {
        Some(parse_backup(&text))
    } else {
        None
    };
    let import_mode = match mode.as_str() {
        "merge" => ImportMode::Merge,
        "replace" => ImportMode::ReplaceProfile(ctx.data.profile().id.clone()),
        _ => ImportMode::Add,
    };
    let plan = if let Some(Ok(p)) = &*preview {
        Some(plan_csv_import(
            &ctx.data,
            p,
            &import_mode,
            *skip,
            &browser::now(),
        ))
    } else if let Some(Ok(b)) = &backup {
        Some(plan_backup(&ctx.data, b, *mode == "replace"))
    } else {
        None
    };
    let executable = plan.as_ref().is_some_and(Result::is_ok)
        && (*mode != "replace" || *confirm)
        && *ctx.ready
        && !*ctx.busy;
    let apply = {
        let ctx = ctx.clone();
        let plan = plan.clone();
        let confirm = confirm.clone();
        let text = text.clone();
        Callback::from(move |_| {
            let Some(Ok(plan)) = &plan else {
                return;
            };
            let text = text.clone();
            let confirm = confirm.clone();
            ctx.save(
                plan.data.clone(),
                &format!(
                    "Импорт завершён: добавлено {}, обновлено {}, пропущено {}",
                    plan.added, plan.updated, plan.skipped
                ),
                Callback::from(move |_| {
                    text.set(String::new());
                    confirm.set(false);
                }),
            );
        })
    };
    html! {<>
        <div class="page-heading"><div><span class="eyebrow">{"КОЛЛЕКЦИЯ БЕЗ ГРАНИЦ"}</span><h1>{"Импорт и экспорт"}</h1><p>{"Заберите свою стаю с собой. Или верните её из файла."}</p></div></div>
        <div class="export-grid">
            <section class="panel export-card"><div class="file-icon">{"CSV"}</div><div><h2>{"Таблица дино"}</h2><p>{"Статы, имена и заметки. Одна строка — один дино. Для таблиц и переноса коллекции."}</p><div class="button-row">
                <button class="button primary" disabled={!*ctx.ready} onclick={let ctx=ctx.clone();Callback::from(move |_|ctx.export_csv(ctx.data.creatures.clone()))}><Icon name="download"/>{"Вся коллекция"}</button>
                <button class="button secondary" disabled={!*ctx.ready} onclick={let ctx=ctx.clone();Callback::from(move |_|ctx.export_csv(ctx.data.creatures.iter().filter(|c|c.profile_id==ctx.data.profile().id).cloned().collect()))}>{"Текущий профиль"}</button>
            </div></div></section>
            <section class="panel export-card"><div class="file-icon json">{"JSON"}</div><div><h2>{"Полная копия"}</h2><p>{"Все дино, профили, настройки, избранное и планы на приручение. Для полного восстановления."}</p>
                <button class="button secondary" disabled={!*ctx.ready} onclick={let ctx=ctx.clone();Callback::from(move |_|ctx.export_json())}><Icon name="download"/>{"Скачать резервную копию"}</button>
            </div></section>
        </div>
        <section class="panel import-panel">
            <div class="section-heading"><div><span class="eyebrow">{"ВОССТАНОВЛЕНИЕ И ПЕРЕНОС"}</span><h2>{"Загрузить коллекцию"}</h2></div><span class="source-tag">{"CSV / JSON"}</span></div>
            <label class="upload-zone"><Icon name="upload"/><strong>{if *loading{"Читаем файл…"}else{"Выберите файл коллекции"}}</strong><span>{"CSV в UTF-8 или резервная копия JSON · до 20 МБ"}</span><input type="file" accept=".csv,.json,text/csv,application/json" onchange={onfile} disabled={*loading||*ctx.busy}/></label>
            if let Some(error)=&*fileerror{<div class="error-box">{error}</div>}
            if !text.is_empty(){
                <div class="import-file"><Icon name="check"/><strong>{&*filename}</strong><span>{format!("{:.1} КБ",text.len() as f64/1024.0)}</span></div>
                if !is_json {
                    <div class="filter-row">
                        <label>{"Разделитель"}<SelectField value={(*delimiter).clone()} onchange={let delimiter=delimiter.clone();let confirm=confirm.clone();Callback::from(move |e:Event|{delimiter.set(e.target_unchecked_into::<HtmlSelectElement>().value());confirm.set(false);})}><option value="auto">{format!("Авто: {}",if guessed==b';'{"точка с запятой"}else{"запятая"})}</option><option value="comma">{"Запятая"}</option><option value="semicolon">{"Точка с запятой"}</option></SelectField></label>
                        <label class="checkbox-label"><input type="checkbox" checked={*decimal} onchange={let decimal=decimal.clone();Callback::from(move |e:Event|decimal.set(e.target_unchecked_into::<HtmlInputElement>().checked()))}/>{"Дробная запятая (для разделителя ;)"}</label>
                    </div>
                    <details class="mapping-details"><summary>{"Соответствие колонок"}</summary><div class="mapping-grid">{for headers.iter().map(|header|{
                        let selected=mapping.get(header).cloned().unwrap_or_else(||canonical_header(header));
                        let onchange={let mapping=mapping.clone();let header=header.clone();Callback::from(move |e:Event|{let mut v=(*mapping).clone();v.insert(header.clone(),e.target_unchecked_into::<HtmlSelectElement>().value());mapping.set(v);})};
                        html!{<label>{header}<SelectField value={selected} onchange={onchange}><option value="">{"Пропустить"}</option>{for CSV_COLUMNS.iter().map(|field|html!{<option value={*field}>{field}</option>})}</SelectField></label>}
                    })}</div></details>
                }
                <div class="import-options"><label>{"Как применить"}<SelectField value={(*mode).clone()} onchange={let mode=mode.clone();let confirm=confirm.clone();Callback::from(move |e:Event|{mode.set(e.target_unchecked_into::<HtmlSelectElement>().value());confirm.set(false);})}>
                    if !is_json{<option value="add">{"Добавить новые · существующие UUID пропустить"}</option>}
                    <option value="merge">{"Объединить · обновить совпавшие UUID"}</option>
                    <option value="replace">{if is_json{"Полностью заменить данные приложения"}else{"Заменить только текущий профиль"}}</option>
                </SelectField></label>
                if !is_json{<label class="checkbox-label"><input type="checkbox" checked={*skip} onchange={let skip=skip.clone();Callback::from(move |e:Event|skip.set(e.target_unchecked_into::<HtmlInputElement>().checked()))}/>{"Импортировать только корректные строки"}</label>}
                </div>
                if let Some(result)=&*preview{
                    {match result{
                        Err(e)=>html!{<div class="error-box" role="alert">{e}</div>},
                        Ok(p)=>html!{<>
                            <div class="preview-summary"><strong>{format!("{} строк",p.rows.len())}</strong><span class={classes!((p.error_count()>0).then_some("error-text"))}>{format!("С ошибками: {}",p.error_count())}</span><span>{format!("Новых профилей: {}",p.profiles.len())}</span></div>
                            {for p.warnings.iter().map(|w|html!{<div class="callout">{w}</div>})}
                            <div class="table-wrap"><table class="import-table"><thead><tr><th>{"Строка"}</th><th>{"Имя / вид"}</th><th>{"Уровень"}</th><th>{"HP"}</th><th>{"Результат проверки"}</th></tr></thead><tbody>
                            {for p.rows.iter().take(20).map(|r|{
                                let resolve=r.errors.iter().any(|e|["creature_id","creature_name"].contains(&e.field.as_str()));
                                html!{<tr><td>{r.line}</td><td><strong>{r.value.title()}</strong><small>{&r.value.reference.creature_name}</small>
                                    if resolve{<SelectField label={format!("Уточнить вид в строке {}",r.line)} value={overrides.get(&r.line).cloned().unwrap_or_default()} onchange={let overrides=overrides.clone();let line=r.line;Callback::from(move |e:Event|{let mut v=(*overrides).clone();v.insert(line,e.target_unchecked_into::<HtmlSelectElement>().value());overrides.set(v);})}>
                                        <option value="">{"Выберите вид"}</option>{for ctx.catalog.creatures.iter().filter(|c|c.game==r.value.reference.game).map(|c|html!{<option value={c.id.clone()}>{format!("{} / {}",c.name,c.pack_id)}</option>})}
                                    </SelectField>}
                                    </td><td>{r.value.level.map(|n|n.to_string()).unwrap_or("—".into())}</td><td>{number(r.value.stats.health)}</td><td>
                                    if r.errors.is_empty(){<span class="source-tag">{if ctx.data.creatures.iter().any(|c|c.id==r.value.id){"UUID уже существует"}else{"Новая запись"}}</span>}
                                    else{{for r.errors.iter().map(|e|html!{<p class="error-text">{format!("{}: {}",e.field,e.message)}</p>})}}
                                    </td></tr>}
                            })}
                            </tbody></table></div>
                            <p class="muted small">{"Предпросмотр первых 20 строк. Применение учитывает все строки файла."}</p>
                            if p.error_count()>0{<button class="button secondary small-button" onclick={let text=export_errors(p);Callback::from(move |_|browser::download(&text,"sataya-import-errors.csv","text/csv;charset=utf-8"))}>{"Скачать ошибки CSV"}</button>}
                        </>}
                    }}
                }
                if let Some(result)=&backup{
                    {match result{
                        Ok(b)=>html!{<div class="callout">{format!("Копия от {}: {} дино, {} профилей. Настройки и списки избранного входят в файл.",b.exported_at,b.data.creatures.len(),b.data.profiles.len())}</div>},
                        Err(e)=>html!{<div class="error-box">{e}</div>}
                    }}
                }
                if let Some(result)=&plan{
                    {match result{
                        Ok(p)=>html!{<div class="import-plan"><strong>{"План изменений"}</strong><span>{format!("Добавить: {} · обновить: {} · пропустить: {} · удалить перед заменой: {}",p.added,p.updated,p.skipped,p.removed)}</span></div>},
                        Err(e)=>html!{<div class="error-box">{e}</div>}
                    }}
                }
                if *mode=="replace"{
                    <div class="replace-confirm"><button class="button secondary small-button" onclick={let ctx=ctx.clone();Callback::from(move |_|ctx.export_json())}>{"Скачать текущую копию JSON"}</button><label class="checkbox-label"><input type="checkbox" checked={*confirm} onchange={let confirm=confirm.clone();Callback::from(move |e:Event|confirm.set(e.target_unchecked_into::<HtmlInputElement>().checked()))}/>{if is_json{"Я проверил предпросмотр и хочу заменить все данные приложения"}else{"Я проверил предпросмотр и хочу заменить дино текущего профиля"}}</label></div>
                }
                <div class="import-footer"><span class="muted small">{"До нажатия «Применить» коллекция остаётся прежней."}</span><button class="button primary" onclick={apply} disabled={!executable}><Icon name="check"/>{if *ctx.busy{"Сохраняем…"}else{"Применить импорт"}}</button></div>
            }
        </section>
    </>}
}
