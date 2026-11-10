use crate::{Route, state::*, ui::*};
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;
use yew_router::prelude::*;

#[component]
pub fn CatalogView() -> Html {
    let ctx = use_app();
    let search = use_state(String::new);
    let method = use_state(String::new);
    let pack = use_state(|| "active".to_owned());
    let tab = use_state(|| "all".to_owned());
    let game = ctx.data.profile().game;
    {
        let pack = pack.clone();
        use_effect_with(ctx.data.profile().id.clone(), move |_| {
            pack.set("active".to_owned());
            || ()
        });
    }
    let onsearch = {
        let v = search.clone();
        Callback::from(move |e: InputEvent| {
            v.set(e.target_unchecked_into::<HtmlInputElement>().value())
        })
    };
    let onmethod = {
        let v = method.clone();
        Callback::from(move |e: Event| {
            v.set(e.target_unchecked_into::<HtmlSelectElement>().value())
        })
    };
    let onpack = {
        let v = pack.clone();
        Callback::from(move |e: Event| {
            v.set(e.target_unchecked_into::<HtmlSelectElement>().value())
        })
    };
    let profile = ctx.data.profile();
    let visible: Vec<_> = ctx
        .catalog
        .creatures
        .iter()
        .filter(|c| {
            c.game == game
                && c.matches(&search)
                && (method.is_empty() || c.guide.method == *method)
                && (pack.is_empty()
                    || c.pack_id == *pack
                    || (*pack == "active" && profile.packs.contains(&c.pack_id)))
                && match tab.as_str() {
                    "owned" => ctx
                        .data
                        .creatures
                        .iter()
                        .any(|o| o.reference.creature_id == c.id && o.profile_id == profile.id),
                    "wish" => ctx.data.preferences.wishlist.contains(&c.id),
                    "favorites" => ctx.data.preferences.favorites.contains(&c.id),
                    _ => true,
                }
        })
        .collect();
    let count = ctx
        .catalog
        .creatures
        .iter()
        .filter(|c| c.game == game && profile.packs.contains(&c.pack_id))
        .count();
    let add = {
        let ctx = ctx.clone();
        Callback::from(move |_| ctx.add(None, false))
    };
    let feature = ctx
        .catalog
        .creatures
        .iter()
        .find(|c| c.game == game && c.slug == "rex" && c.pack_id == "vanilla");
    html! {<>
        <div class="page-heading"><div><div class="eyebrow">{"ПОЛЕВОЙ СПРАВОЧНИК / "}{game.key().to_uppercase()}</div><h1>{"Каталог существ"}<span class="count-pill">{count}</span></h1><p>{"От первого додо до нового пополнения стаи."}</p></div><button class="button primary" onclick={add}><Icon name="plus"/>{"Добавить дино"}</button></div>
        if let Some(c)=feature {
            <section class="feature-card"><div class="feature-content"><div class="eyebrow">{"ЗНАКОМЫЙ ВИД. НОВАЯ ИСТОРИЯ."}</div><h2>{"Король острова."}</h2><p>{"Рекс — сильное начало вашей боевой стаи. Подготовьте ловушку, припасы и план отхода."}</p><Link<Route> classes="button dark" to={Route::Creature{id:c.id.clone()}}>{"Как приручить"}<Icon name="chevron"/></Link<Route>><div class="feature-meta"><span>{"TYRANNOSAURUS"}</span><span>{"THE ISLAND / 001"}</span></div></div><div class="feature-illustration"><div class="orbit one"/><div class="orbit two"/><span class="crosshair top">{"+"}</span><DinoArt kind="theropod"/><span class="specimen-label">{"SPECIMEN · REX"}</span></div></section>
        }
        <div class="catalog-tools"><div class="tabs">{for [("all","Все существа"),("owned","Мои дино"),("wish","Хочу приручить"),("favorites","Избранное")].iter().map(|(key,label)|{
            let selected=*tab==*key;let tab=tab.clone();let key=key.to_string();html!{<button class={classes!("tab",selected.then_some("active"))} onclick={Callback::from(move |_|tab.set(key.clone()))}>{label}</button>}
        })}</div><span class="muted small">{format!("{} записей",visible.len())}</span></div>
        <div class="filter-row"><label class="search-field"><Icon name="search"/><input aria-label="Поиск существ" placeholder="Название на русском или английском…" value={(*search).clone()} oninput={onsearch}/><kbd>{"/"}</kbd></label><SelectField label="Способ приручения" value={(*method).clone()} onchange={onmethod}><option value="">{"Все способы"}</option><option value="knockout">{"Оглушение"}</option><option value="passive">{"Пассивное"}</option><option value="special">{"Особый способ"}</option></SelectField><SelectField label="Пакет существ" value={(*pack).clone()} onchange={onpack}><option value="active">{"Пакеты профиля"}</option><option value="">{"Все пакеты"}</option>{for ctx.catalog.packs.iter().filter(|p|p.game==game&&p.available).map(|p|html!{<option value={p.id.clone()}>{&p.name}</option>})}</SelectField></div>
        if visible.is_empty(){<div class="empty-state"><Icon name="search"/><h2>{"Никого не нашли"}</h2><p>{"Попробуйте другое название или измените фильтры."}</p></div>}
        <div class="creature-grid">{for visible.iter().map(|c|{
            let owned=ctx.data.creatures.iter().filter(|o|o.reference.creature_id==c.id&&o.profile_id==profile.id).count();
            let favorite=ctx.data.preferences.favorites.contains(&c.id);let toggle={let ctx=ctx.clone();let id=c.id.clone();Callback::from(move |_|ctx.bookmark(id.clone(),false))};
            html!{<article class="creature-card" key={c.id.clone()}><div class={classes!("card-art",format!("art-{}",c.art))}><span class={classes!("method-badge",format!("method-{}",c.guide.method))}><span class="status-dot"/>{method_label(&c.guide.method)}</span><button class={classes!("favorite-button",favorite.then_some("selected"))} onclick={toggle} aria-label={if favorite{"Убрать из избранного"}else{"В избранное"}} disabled={!*ctx.ready}><Icon name="heart"/></button><Link<Route> to={Route::Creature{id:c.id.clone()}}><DinoArt kind={c.art.clone()}/></Link<Route>></div><div class="card-body"><div class="card-subtitle">{&c.name_en}</div><Link<Route> classes="card-title" to={Route::Creature{id:c.id.clone()}}>{&c.name}</Link<Route>><div class="card-description">{&c.role}</div><div class="card-footer"><span class={classes!("source-tag",(c.pack_id!="vanilla").then_some("mod"))}>{if c.pack_id=="vanilla"{"Базовая игра"}else{c.pack_id.as_str()}}</span><span class={classes!("owned-count",(owned>0).then_some("has-owned"))}>{if owned>0{format!("В коллекции: {owned}")}else{"Не приручён".into()}}</span></div></div></article>}
        })}</div><div class="page-footnote"><span class="status-dot"/>{"Инструкции и источники хранятся вместе со справочником. Личная коллекция — в вашем браузере."}</div>
    </>}
}

#[derive(Properties, PartialEq)]
pub struct DetailProps {
    pub id: String,
}

#[component]
pub fn CreatureView(props: &DetailProps) -> Html {
    let ctx = use_app();
    let Some(c) = ctx.catalog.creatures.iter().find(|c| c.id == props.id) else {
        return html! {<div class="empty-state"><h1>{"Карточка не найдена"}</h1><Link<Route> to={Route::Catalog}>{"Вернуться в каталог"}</Link<Route>></div>};
    };
    let add = {
        let ctx = ctx.clone();
        let id = c.id.clone();
        Callback::from(move |_| ctx.add(Some(id.clone()), false))
    };
    let shot = {
        let ctx = ctx.clone();
        let id = c.id.clone();
        Callback::from(move |_| ctx.add(Some(id.clone()), true))
    };
    let wish = ctx.data.preferences.wishlist.contains(&c.id);
    let toggle = {
        let ctx = ctx.clone();
        let id = c.id.clone();
        Callback::from(move |_| ctx.bookmark(id.clone(), true))
    };
    let owned = ctx
        .data
        .creatures
        .iter()
        .filter(|o| o.reference.creature_id == c.id)
        .count();
    html! {<>
        <Link<Route> classes="back-link" to={Route::Catalog}><Icon name="back"/>{"Каталог существ"}</Link<Route>>
        <section class="detail-hero"><div><span class="eyebrow">{format!("{} / {}",c.game.key().to_uppercase(),c.pack_id.to_uppercase())}</span><h1>{&c.name}</h1><p class="latin">{&c.name_en}</p><div class="detail-tags"><span>{&c.group}</span><span>{&c.diet}</span><span>{&c.role}</span></div></div><DinoArt kind={c.art.clone()}/></section>
        <div class="detail-layout"><div><section class="panel intro"><h2>{"Знакомство с видом"}</h2><p>{&c.description}</p></section><section class="panel"><div class="section-heading"><h2>{"Как приручить"}</h2><span class="source-tag">{method_label(&c.guide.method)}</span></div>
        if c.guide.partial { <div class="callout">{"Инструкция для этого варианта ещё дополняется. Перед приручением проверьте страницу автора."}</div> }
        <ol class="taming-steps">{for c.guide.steps.iter().enumerate().map(|(i,step)|html!{<li><span class="step-number">{format!("{:02}",i+1)}</span><p>{step}</p></li>})}</ol>
        if !c.guide.notes.is_empty(){<div class="guide-notes"><h3>{"Полевые заметки"}</h3>{for c.guide.notes.iter().map(|n|html!{<p>{n}</p>})}</div>}
        </section><div class="source-line"><a href={c.guide.source.clone()} target="_blank" rel="noopener noreferrer">{"Источник инструкции"}<Icon name="external"/></a><span>{format!("Редакция {} · {}",c.revision,c.guide.checked_at)}</span></div></div>
        <aside class="detail-side"><section class="panel"><span class="eyebrow">{"ВАША СТАЯ"}</span><h2>{if owned>0{format!("В коллекции: {owned}")}else{"Первый представитель?".into()}}</h2><p class="muted">{"Добавьте своего дино и сохраните его характеристики."}</p><button class="button primary full" onclick={add}><Icon name="plus"/>{"Добавить вручную"}</button><button class="button secondary full" onclick={shot}><Icon name="photo"/>{"Добавить по скрину"}</button><button class={classes!("button","ghost","full",wish.then_some("selected"))} onclick={toggle}><Icon name="star"/>{if wish{"В планах на приручение"}else{"Хочу приручить"}}</button></section><section class="panel"><h3>{"Возьмите с собой"}</h3><span class="field-label">{"Корм"}</span><div class="food-tags">{for c.guide.food.iter().map(|f|html!{<span>{f}</span>})}</div><span class="field-label">{"Снаряжение"}</span><ul class="plain-list">{for c.guide.equipment.iter().map(|f|html!{<li><Icon name="check"/>{f}</li>})}</ul><span class="field-label">{"Карта"}</span><p>{c.maps.join(", ")}</p></section></aside></div>
    </>}
}
