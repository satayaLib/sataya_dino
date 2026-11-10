use crate::{Route, state::*, ui::*};
use ark_core::*;
use std::cmp::Ordering;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;
use yew_router::prelude::*;

#[component]
pub fn CollectionView() -> Html {
    let ctx = use_app();
    let search = use_state(String::new);
    let status = use_state(String::new);
    let sex = use_state(String::new);
    let sort = use_state(|| "updated".to_owned());
    let page = use_state(|| 0usize);
    let remove = use_state(|| None::<OwnedCreature>);
    let profile = ctx.data.profile();
    let mut rows: Vec<_> = ctx
        .data
        .creatures
        .iter()
        .filter(|c| {
            c.profile_id == profile.id
                && (status.is_empty() || c.status == *status)
                && (sex.is_empty() || c.sex == *sex)
                && (c.title().to_lowercase().contains(&search.to_lowercase())
                    || c.reference
                        .creature_name
                        .to_lowercase()
                        .contains(&search.to_lowercase())
                    || c.tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&search.to_lowercase())))
        })
        .cloned()
        .collect();
    rows.sort_by(|a, b| match sort.as_str() {
        "name" => a.title().cmp(b.title()),
        "updated" => b.updated_at.cmp(&a.updated_at),
        key => {
            let av = if key == "level" {
                a.level.map(|v| v as f64)
            } else {
                a.stats.get(key)
            };
            let bv = if key == "level" {
                b.level.map(|v| v as f64)
            } else {
                b.stats.get(key)
            };
            match (av, bv) {
                (Some(a), Some(b)) => b.total_cmp(&a),
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                _ => Ordering::Equal,
            }
        }
    });
    let onsearch = {
        let state = search.clone();
        let page = page.clone();
        Callback::from(move |e: InputEvent| {
            page.set(0);
            state.set(e.target_unchecked_into::<HtmlInputElement>().value());
        })
    };
    let onstatus = {
        let state = status.clone();
        let page = page.clone();
        Callback::from(move |e: Event| {
            page.set(0);
            state.set(e.target_unchecked_into::<HtmlSelectElement>().value());
        })
    };
    let onsex = {
        let state = sex.clone();
        let page = page.clone();
        Callback::from(move |e: Event| {
            page.set(0);
            state.set(e.target_unchecked_into::<HtmlSelectElement>().value());
        })
    };
    let onsort = {
        let state = sort.clone();
        let page = page.clone();
        Callback::from(move |e: Event| {
            page.set(0);
            state.set(e.target_unchecked_into::<HtmlSelectElement>().value());
        })
    };
    let add = {
        let ctx = ctx.clone();
        Callback::from(move |_| ctx.add(None, false))
    };
    let shot = {
        let ctx = ctx.clone();
        Callback::from(move |_| ctx.add(None, true))
    };
    let export = {
        let ctx = ctx.clone();
        let rows = rows.clone();
        Callback::from(move |_| ctx.export_csv(rows.clone()))
    };
    let count = ctx
        .data
        .creatures
        .iter()
        .filter(|c| c.profile_id == profile.id)
        .count();
    let start = (*page * 50).min(rows.len().saturating_sub(1) / 50 * 50);
    html! {<>
        <div class="page-heading"><div><span class="eyebrow">{"ВАША СТАЯ / "}{profile.game.key().to_uppercase()}</span><h1>{"Моя коллекция"}<span class="count-pill">{count}</span></h1><p>{format!("{} · всё самое важное о ваших дино",profile.name)}</p></div><div class="button-row"><button class="button secondary" onclick={shot}><Icon name="photo"/>{"По скриншоту"}</button><button class="button primary" onclick={add.clone()}><Icon name="plus"/>{"Добавить дино"}</button></div></div>
        <div class="collection-summary"><div><span class="eyebrow">{"ВСЕГО ДИНО"}</span><strong>{count}</strong></div><div><span class="eyebrow">{"АКТИВНЫХ"}</span><strong>{ctx.data.creatures.iter().filter(|c|c.profile_id==profile.id&&c.status=="active").count()}</strong></div><div><span class="eyebrow">{"ВИДОВ В СТАЕ"}</span><strong>{ctx.data.creatures.iter().filter(|c|c.profile_id==profile.id).map(|c|&c.reference.creature_id).collect::<std::collections::BTreeSet<_>>().len()}</strong></div><div class="summary-note"><Icon name="box"/><span>{"Свои дино. Свои записи."}<small>{"Хранятся на этом устройстве"}</small></span></div></div>
        <div class="filter-row"><label class="search-field"><Icon name="search"/><input aria-label="Поиск в коллекции" placeholder="Имя, вид или тег…" value={(*search).clone()} oninput={onsearch}/></label><SelectField label="Статус дино" value={(*status).clone()} onchange={onstatus}><option value="">{"Все статусы"}</option>{for ["active","cryopod","dead","transferred"].map(|s|html!{<option value={s}>{status_label(s)}</option>})}</SelectField><SelectField label="Пол дино" value={(*sex).clone()} onchange={onsex}><option value="">{"Любой пол"}</option><option value="male">{"Самец"}</option><option value="female">{"Самка"}</option><option value="unknown">{"Не указан"}</option></SelectField><SelectField label="Сортировка коллекции" value={(*sort).clone()} onchange={onsort}><option value="updated">{"По обновлению"}</option><option value="name">{"По имени"}</option><option value="level">{"По уровню"}</option><option value="health">{"По здоровью"}</option><option value="melee_percent">{"По урону"}</option><option value="weight">{"По весу"}</option></SelectField></div>
        <div class="table-toolbar"><span class="muted small">{format!("Показано {} из {}",rows.len(),count)}</span><div class="button-row"><Link<Route> classes={classes!("button","small-button","secondary")} to={Route::Compare}><Icon name="compare"/>{format!("Сравнить ({}/4)",ctx.compare.len())}</Link<Route>><button class="button small-button secondary" onclick={export} disabled={rows.is_empty()}><Icon name="download"/>{"CSV выборки"}</button></div></div>
        if rows.is_empty() {<div class="empty-state collection-empty"><div class="empty-art"><DinoArt kind="sauropod"/></div><span class="eyebrow">{"НАЧАЛО БОЛЬШОЙ ИСТОРИИ"}</span><h2>{if count==0{"Здесь будет ваша стая"}else{"По этим фильтрам никого нет"}}</h2><p>{if count==0{"Добавьте первого дино вручную или загрузите скриншот его характеристик."}else{"Измените фильтры, чтобы увидеть остальных дино."}}</p><button class="button primary" onclick={add}><Icon name="plus"/>{"Добавить первого дино"}</button><Link<Route> classes="text-link" to={Route::Transfer}>{"У меня уже есть CSV или JSON"}</Link<Route>></div>}
        else {<div class="table-wrap"><table class="collection-table"><thead><tr><th aria-label="Сравнить"/><th>{"Дино"}</th><th>{"Пол / ур."}</th><th>{"Здоровье"}</th><th>{"Выносл."}</th><th>{"Урон, %"}</th><th>{"Статус"}</th><th aria-label="Действия"/></tr></thead><tbody>{for rows.iter().skip(start).take(50).map(|c|{
            let art=ctx.catalog.creatures.iter().find(|d|d.id==c.reference.creature_id).map(|d|d.art.as_str()).unwrap_or("theropod");
            let selected=ctx.compare.contains(&c.id);let choose={let ctx=ctx.clone();let id=c.id.clone();Callback::from(move |_:Event|{let mut v=(*ctx.compare).clone();if v.contains(&id){v.retain(|x|x!=&id);}else if v.len()<4{v.push(id.clone());}else{ctx.notice.set(Some("Для сравнения можно выбрать до четырёх дино".into()));}ctx.compare.set(v);})};
            let edit={let ctx=ctx.clone();let id=c.id.clone();Callback::from(move |_|ctx.editor.set(Some(EditorTarget::Edit(id.clone()))))};
            let copy={let ctx=ctx.clone();let id=c.id.clone();Callback::from(move |_|ctx.editor.set(Some(EditorTarget::Copy(id.clone()))))};
            let delete={let remove=remove.clone();let c=c.clone();Callback::from(move |_|remove.set(Some(c.clone())))};
            html!{<tr key={c.id.clone()}><td><input type="checkbox" checked={selected} onchange={choose} aria-label={format!("Сравнить {}",c.title())}/></td><td><div class="dino-cell"><div class="row-art"><DinoArt kind={art.to_owned()}/></div><div><button class="name-button" onclick={edit}>{c.title()}</button><small>{&c.reference.creature_name}{if !ctx.catalog.creatures.iter().any(|d|d.id==c.reference.creature_id){" · пакет недоступен"}else{""}}</small></div></div></td><td><span class="sex">{match c.sex.as_str(){"male"=>"♂","female"=>"♀",_=>"—"}}</span>{c.level.map(|v|v.to_string()).unwrap_or("—".into())}</td><td class="numeric">{number(c.stats.health)}</td><td class="numeric">{number(c.stats.stamina)}</td><td class="numeric">{number(c.stats.melee_percent)}</td><td><span class={classes!("source-tag",format!("status-{}",c.status))}>{status_label(&c.status)}</span></td><td><div class="row-actions"><button class="icon-button" onclick={copy} aria-label={format!("Копировать {}",c.title())}><Icon name="copy"/></button><button class="icon-button" onclick={delete} aria-label={format!("Удалить {}",c.title())}><Icon name="trash"/></button></div></td></tr>}
        })}</tbody></table></div><div class="pagination"><span>{format!("{}–{} из {}",start+1,(start+50).min(rows.len()),rows.len())}</span><div class="button-row"><button class="button secondary small-button" disabled={start==0} onclick={let page=page.clone();Callback::from(move |_|page.set(start.saturating_sub(50)/50))}>{"Назад"}</button><button class="button secondary small-button" disabled={start+50>=rows.len()} onclick={let page=page.clone();Callback::from(move |_|page.set(start/50+1))}>{"Далее"}</button></div></div>}
        if let Some(c)=&*remove {<Modal title="Удалить дино?" onclose={let remove=remove.clone();Callback::from(move |_|remove.set(None))}><div class="modal-body"><p>{format!("{} будет удалён из коллекции. Остальные дино останутся на месте.",c.title())}</p></div><footer class="modal-footer"><button class="button secondary" onclick={let remove=remove.clone();Callback::from(move |_|remove.set(None))}>{"Отмена"}</button><button class="button danger" disabled={*ctx.busy} onclick={let ctx=ctx.clone();let id=c.id.clone();let remove=remove.clone();Callback::from(move |_|{let mut next=(*ctx.data).clone();next.creatures.retain(|c|c.id!=id);let remove=remove.clone();ctx.save(next,"Дино удалён",Callback::from(move |_|remove.set(None)));})}>{"Удалить"}</button></footer></Modal>}
    </>}
}

#[component]
pub fn CompareView() -> Html {
    let ctx = use_app();
    let rows: Vec<_> = ctx
        .compare
        .iter()
        .filter_map(|id| ctx.data.creatures.iter().find(|c| &c.id == id))
        .collect();
    html! {<><div class="page-heading"><div><span class="eyebrow">{"ХАРАКТЕРИСТИКИ РЯДОМ"}</span><h1>{"Сравнение дино"}</h1><p>{"Отображаемые статы с контекстом прокачки и импринта."}</p></div><Link<Route> classes="button secondary" to={Route::Collection}><Icon name="back"/>{"В коллекцию"}</Link<Route>></div>
        if rows.is_empty(){<div class="empty-state"><Icon name="compare"/><h2>{"Выберите дино для сравнения"}</h2><p>{"Отметьте до четырёх записей в коллекции."}</p></div>}
        else{<div class="table-wrap"><table class="comparison"><thead><tr><th>{"Характеристика"}</th>{for rows.iter().map(|c|html!{<th><strong>{c.title()}</strong><small>{&c.reference.creature_name}</small><span class="source-tag">{format!("{} / {}",c.reference.game.key().to_uppercase(),c.reference.pack_id)}</span></th>})}</tr></thead><tbody>
            <tr><th>{"Уровень"}</th>{for rows.iter().map(|c|html!{<td>{c.level.map(|v|v.to_string()).unwrap_or("—".into())}</td>})}</tr>
            <tr><th>{"Стадия статов"}</th>{for rows.iter().map(|c|html!{<td>{match c.stat_stage.as_str(){"after_tame"=>"После приручения","newborn"=>"При рождении",_=>"После прокачки"}}</td>})}</tr>
            {for STAT_FIELDS.iter().map(|(key,label,unit)|{let max=rows.iter().filter_map(|c|c.stats.get(key)).max_by(f64::total_cmp);html!{<tr><th>{label}{if unit.is_empty(){String::new()}else{format!(", {unit}")}}</th>{for rows.iter().map(|c|{let value=c.stats.get(key);html!{<td class={classes!((value.is_some()&&value==max).then_some("best-value"))}>{number(value)}</td>}})}</tr>}})}
            <tr><th>{"Профиль"}</th>{for rows.iter().map(|c|html!{<td>{ctx.data.profiles.iter().find(|p|p.id==c.profile_id).map(|p|p.name.clone()).unwrap_or_default()}</td>})}</tr>
        </tbody></table></div><p class="muted small">{"Выделено наибольшее введённое значение. Разные виды, варианты, стадии прокачки и импринт влияют на сопоставимость статов."}</p>}
    </>}
}
