use crate::{Route, browser, state::*, ui::*};
use ark_core::*;
use serde::Deserialize;
use std::collections::BTreeMap;
use wasm_bindgen::{JsCast, closure::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement, SubmitEvent};
use yew::prelude::*;
use yew_router::prelude::*;

#[component]
pub fn PacksView() -> Html {
    let ctx = use_app();
    let game = ctx.data.profile().game;
    html! {<><div class="page-heading"><div><span class="eyebrow">{"ДОПОЛНЕНИЯ К СПРАВОЧНИКУ"}</span><h1>{"Пакеты существ"}</h1><p>{format!("Выберите пакеты для профиля «{}».",ctx.data.profile().name)}</p></div></div><div class="callout">{"Варианты из модов имеют отдельные карточки и инструкции. Список пакетов — настройка справочника, а не проверка совместимости установленных модов."}</div><div class="packs-grid">{for ctx.catalog.packs.iter().filter(|p|p.game==game).map(|p|{
        let selected=ctx.data.profile().packs.contains(&p.id);let count=ctx.catalog.creatures.iter().filter(|c|c.game==game&&c.pack_id==p.id).count();
        let change={let ctx=ctx.clone();let id=p.id.clone();Callback::from(move |_:Event|{let mut next=(*ctx.data).clone();let active=next.preferences.active_profile_id.clone();if let Some(profile)=next.profiles.iter_mut().find(|p|p.id==active)&& !profile.packs.remove(&id){profile.packs.insert(id.clone());}ctx.save(next,"Пакеты профиля обновлены",Callback::noop());})};
        html!{<section class={classes!("panel","pack-card",(!p.available).then_some("planned"))}><div class="pack-icon"><Icon name="box"/></div><span class="eyebrow">{game.key().to_uppercase()}</span><h2>{&p.name}</h2><p>{&p.description}</p><div class="pack-meta"><span class="source-tag">{if p.available{format!("Карточек: {count}")}else{"В очереди наполнения".into()}}</span><a href={p.source.clone()} target="_blank" rel="noopener noreferrer" aria-label={format!("Источник {}",p.name)}><Icon name="external"/></a></div><label class="checkbox-label"><input type="checkbox" checked={selected} onchange={change} disabled={!p.available||p.id=="vanilla"||!*ctx.ready||*ctx.busy}/>{if p.id=="vanilla"{"Включён всегда"}else if selected{"Включён в профиле"}else if p.available{"Включить в профиле"}else{"Данные ещё готовятся"}}</label></section>}
    })}</div></>}
}

#[component]
pub fn ProfilesView() -> Html {
    let ctx = use_app();
    let edit = use_state(|| None::<Profile>);
    let remove = use_state(|| None::<Profile>);
    html! {<><div class="page-heading"><div><span class="eyebrow">{"МИРЫ И СЕРВЕРЫ"}</span><h1>{"Ваши профили"}</h1><p>{"Раздельные коллекции для разных серверов и прохождений."}</p></div><button class="button primary" onclick={let edit=edit.clone();let game=ctx.data.profile().game;Callback::from(move |_|edit.set(Some(Profile::new(browser::new_id(),String::new(),game))))}><Icon name="plus"/>{"Новый профиль"}</button></div><div class="profiles-grid">{for ctx.data.profiles.iter().map(|p|{
        let active=p.id==ctx.data.preferences.active_profile_id;let count=ctx.data.creatures.iter().filter(|c|c.profile_id==p.id).count();
        let choose={let ctx=ctx.clone();let id=p.id.clone();Callback::from(move |_|{let mut next=(*ctx.data).clone();next.preferences.active_profile_id=id.clone();ctx.save(next,"Профиль выбран",Callback::noop());})};
        html!{<section class={classes!("panel","profile-card",active.then_some("active"))}><div class="section-heading"><span class="source-tag">{p.game.key().to_uppercase()}</span>if active{<span class="active-label"><span class="status-dot"/>{"Текущий профиль"}</span>}</div><h2>{&p.name}</h2><p>{format!("{} · {} дино",p.map,count)}</p><div class="profile-rates"><span>{format!("Приручение ×{}",p.taming_multiplier)}</span><span>{format!("Расход еды ×{}",p.food_multiplier)}</span></div><div class="button-row"><button class="button secondary small-button" onclick={choose} disabled={active||*ctx.busy||!*ctx.ready}>{if active{"Выбран"}else{"Выбрать"}}</button><button class="button ghost small-button" onclick={let edit=edit.clone();let p=p.clone();Callback::from(move |_|edit.set(Some(p.clone())))}>{"Настроить"}</button><button class="icon-button" aria-label={format!("Удалить профиль {}",p.name)} disabled={ctx.data.profiles.len()<2||*ctx.busy} onclick={let remove=remove.clone();let p=p.clone();Callback::from(move |_|remove.set(Some(p.clone())))}><Icon name="trash"/></button></div></section>}
    })}</div>
    if let Some(profile)=&*edit {<ProfileEditor key={profile.id.clone()} profile={profile.clone()} onclose={let edit=edit.clone();Callback::from(move |_|edit.set(None))}/>}
    if let Some(profile)=&*remove {<Modal title="Удалить профиль?" onclose={let remove=remove.clone();Callback::from(move |_|remove.set(None))}><div class="modal-body"><p>{format!("Из профиля «{}» будут удалены все дино ({}). Остальные профили сохранятся.",profile.name,ctx.data.creatures.iter().filter(|c|c.profile_id==profile.id).count())}</p><button class="button secondary" onclick={let ctx=ctx.clone();Callback::from(move |_|ctx.export_json())}><Icon name="download"/>{"Сначала скачать копию JSON"}</button></div><footer class="modal-footer"><button class="button secondary" onclick={let remove=remove.clone();Callback::from(move |_|remove.set(None))}>{"Отмена"}</button><button class="button danger" disabled={*ctx.busy} onclick={let ctx=ctx.clone();let id=profile.id.clone();let remove=remove.clone();Callback::from(move |_|{let mut next=(*ctx.data).clone();next.profiles.retain(|p|p.id!=id);next.creatures.retain(|c|c.profile_id!=id);if next.preferences.active_profile_id==id{next.preferences.active_profile_id=next.profiles[0].id.clone();}let remove=remove.clone();ctx.save(next,"Профиль удалён",Callback::from(move |_|remove.set(None)));})}>{"Удалить профиль и его дино"}</button></footer></Modal>}
    </>}
}

#[derive(Properties, PartialEq)]
struct ProfileEditorProps {
    profile: Profile,
    onclose: Callback<()>,
}

#[component]
fn ProfileEditor(props: &ProfileEditorProps) -> Html {
    let ctx = use_app();
    let p = use_state(|| props.profile.clone());
    let errors = use_state(|| None::<String>);
    let rates = use_state(|| {
        BTreeMap::from([
            ("taming", p.taming_multiplier.to_string()),
            ("food", p.food_multiplier.to_string()),
            ("stat", p.stat_multiplier.to_string()),
        ])
    });
    let existing = ctx.data.profiles.iter().any(|old| old.id == p.id);
    let submit = {
        let ctx = ctx.clone();
        let p = p.clone();
        let rates = rates.clone();
        let errors = errors.clone();
        let onclose = props.onclose.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let mut p = (*p).clone();
            for (key, value) in rates.iter() {
                let Ok(Some(value)) = parse_decimal(value, value.contains(',')) else {
                    errors.set(Some("Проверьте множители".into()));
                    return;
                };
                if value <= 0.0 {
                    errors.set(Some("Множители должны быть больше нуля".into()));
                    return;
                }
                match *key {
                    "taming" => p.taming_multiplier = value,
                    "food" => p.food_multiplier = value,
                    _ => p.stat_multiplier = value,
                }
            }
            let mut next = (*ctx.data).clone();
            if let Some(old) = next.profiles.iter_mut().find(|old| old.id == p.id) {
                *old = p;
            } else {
                next.preferences.active_profile_id = p.id.clone();
                next.profiles.push(p);
            }
            if let Err(e) = next.validate() {
                errors.set(Some(e));
                return;
            }
            ctx.save(next, "Профиль сохранён", onclose.clone());
        })
    };
    html! {<Modal title={if existing{"Настройки профиля"}else{"Новый профиль"}} onclose={props.onclose.clone()}><form onsubmit={submit}><div class="modal-body form-grid"><label class="span-2">{"Название"}<input required={true} value={p.name.clone()} oninput={let p=p.clone();Callback::from(move |e:InputEvent|{let mut v=(*p).clone();v.name=e.target_unchecked_into::<HtmlInputElement>().value();p.set(v);})}/></label><label>{"Игра"}<SelectField disabled={existing} value={p.game.key()} onchange={let p=p.clone();Callback::from(move |e:Event|{let mut v=(*p).clone();v.game=if e.target_unchecked_into::<HtmlSelectElement>().value()=="ase"{Game::Ase}else{Game::Asa};p.set(v);})}><option value="asa">{"Survival Ascended"}</option><option value="ase">{"Survival Evolved"}</option></SelectField></label><label>{"Карта по умолчанию"}<input value={p.map.clone()} oninput={let p=p.clone();Callback::from(move |e:InputEvent|{let mut v=(*p).clone();v.map=e.target_unchecked_into::<HtmlInputElement>().value();p.set(v);})}/></label>{for[("taming","Скорость приручения"),("food","Расход еды дино"),("stat","Множитель статов")].map(|(key,label)|html!{<label>{label}<input inputmode="decimal" value={rates.get(key).cloned().unwrap_or_default()} oninput={let rates=rates.clone();Callback::from(move |e:InputEvent|{let mut v=(*rates).clone();v.insert(key,e.target_unchecked_into::<HtmlInputElement>().value());rates.set(v);})}/></label>})}<label class="checkbox-label"><input type="checkbox" checked={p.single_player} onchange={let p=p.clone();Callback::from(move |e:Event|{let mut v=(*p).clone();v.single_player=e.target_unchecked_into::<HtmlInputElement>().checked();p.set(v);})}/>{"Настройки одиночной игры"}</label><p class="muted small span-2">{"Рейты сохраняются вместе с профилем. Калькуляторы приручения будут учитывать их в следующем этапе."}</p>if let Some(error)=&*errors{<div class="error-box span-2">{error}</div>}</div><footer class="modal-footer"><button type="button" class="button secondary" onclick={let cb=props.onclose.clone();Callback::from(move |_|cb.emit(()))}>{"Отмена"}</button><button class="button primary" type="submit" disabled={*ctx.busy||!*ctx.ready}>{"Сохранить профиль"}</button></footer></form></Modal>}
}

#[derive(Clone, Deserialize, Default, PartialEq)]
struct StorageInfo {
    usage: f64,
    quota: f64,
    persistent: bool,
    online: bool,
}

#[component]
pub fn SettingsView() -> Html {
    let ctx = use_app();
    let info = use_state(StorageInfo::default);
    let offline = use_state(String::new);
    let preparing = use_state(|| false);
    {
        let info = info.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                if let Ok(text) = browser::storage_info().await
                    && let Ok(value) = serde_json::from_str(&text)
                {
                    info.set(value);
                }
            });
            || ()
        });
    }
    let persistence = {
        let info = info.clone();
        let notice = ctx.notice.clone();
        Callback::from(move |_| {
            let info = info.clone();
            let notice = notice.clone();
            spawn_local(async move {
                match browser::request_persistence().await{Ok(true)=>notice.set(Some("Постоянное хранение разрешено браузером".into())),Ok(false)=>notice.set(Some("Браузер пока не разрешил постоянное хранение. Коллекция доступна; сохраняйте резервные копии.".into())),Err(e)=>notice.set(Some(browser::error(e)))}
                if let Ok(text) = browser::storage_info().await
                    && let Ok(value) = serde_json::from_str(&text)
                {
                    info.set(value);
                }
            });
        })
    };
    let prepare = {
        let offline = offline.clone();
        let preparing = preparing.clone();
        Callback::from(move |_| {
            let offline = offline.clone();
            let preparing = preparing.clone();
            preparing.set(true);
            spawn_local(async move {
                let status = offline.clone();
                let progress = Closure::<dyn FnMut(String)>::new(move |s| status.set(s));
                match browser::prepare_offline(progress.as_ref().unchecked_ref()).await {
                    Ok(s) => offline.set(s),
                    Err(e) => offline.set(browser::error(e)),
                }
                preparing.set(false);
            });
        })
    };
    let theme = {
        let ctx = ctx.clone();
        Callback::from(move |_| {
            let mut next = (*ctx.data).clone();
            next.preferences.theme = if next.preferences.theme == "dark" {
                "light".into()
            } else {
                "dark".into()
            };
            ctx.save(next, "Тема обновлена", Callback::noop());
        })
    };
    html! {<><div class="page-heading"><div><span class="eyebrow">{"НА ВАШЕМ УСТРОЙСТВЕ"}</span><h1>{"Настройки"}</h1><p>{"Хранение, резервные копии и работа без сети."}</p></div></div><div class="settings-grid"><section class="panel"><div class="section-heading"><h2>{"Локальное хранилище"}</h2><Icon name="box"/></div><div class="setting-line"><span>{"Сохранённых дино"}</span><strong>{ctx.data.creatures.len()}</strong></div><div class="setting-line"><span>{"Использовано сайтом"}</span><strong>{format!("{:.1} МБ",info.usage/1048576.0)}</strong></div><div class="setting-line"><span>{"Доступная квота"}</span><strong>{format!("{:.0} МБ",info.quota/1048576.0)}</strong></div><div class="setting-line"><span>{"Постоянное хранение"}</span><span class="source-tag">{if info.persistent{"Разрешено"}else{"Обычный режим"}}</span></div><p class="muted small">{"Коллекция привязана к этому браузеру и адресу сайта. Для переноса на другое устройство используйте CSV или JSON."}</p><button class="button secondary" onclick={persistence}>{"Запросить постоянное хранение"}</button></section>
    <section class="panel"><div class="section-heading"><h2>{"Всегда под рукой"}</h2><Icon name="globe"/></div><p>{"Скачайте локальные файлы распознавания и языковые модели. После подготовки добавлять дино по скрину можно без интернета."}</p><span class="source-tag">{if info.online{"Сеть доступна"}else{"Без сети"}}</span><p class="muted small">{"Первая загрузка может занять время. Приложение не отправляет скриншоты внешним сервисам."}</p><button class="button primary" onclick={prepare} disabled={*preparing}>{if *preparing{"Подготавливаем…"}else{"Подготовить к работе без сети"}}</button>if !offline.is_empty(){<p role="status">{&*offline}</p>}</section>
    <section class="panel"><h2>{"Резервные копии"}</h2><p>{"CSV — перенос и правка таблицы дино. JSON — вся коллекция, профили, избранное и настройки."}</p><div class="button-row"><button class="button secondary" onclick={let ctx=ctx.clone();Callback::from(move |_|ctx.export_json())}><Icon name="download"/>{"Скачать JSON"}</button><Link<Route> classes="button ghost" to={Route::Transfer}>{"Импорт и экспорт"}<Icon name="chevron"/></Link<Route>></div><p class="muted small">{format!("Последняя выгрузка CSV: {}",ctx.data.preferences.last_export_at.as_deref().unwrap_or("ещё не было"))}</p></section>
    <section class="panel"><h2>{"Оформление"}</h2><p>{"Выберите удобную тему для заметок и подготовки к следующему приручению."}</p><button class="button secondary" onclick={theme} disabled={!*ctx.ready}><Icon name="moon"/>{if ctx.data.preferences.theme=="dark"{"Светлая тема"}else{"Тёмная тема"}}</button><p class="muted small">{"Sataya Fieldbook · версия 0.1 · независимый справочник по ARK"}</p></section></div></>}
}
