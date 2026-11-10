mod browser;
mod catalog;
mod collection;
mod editor;
mod ocr_view;
mod settings;
mod state;
mod storage;
mod transfer;
mod ui;

use ark_core::*;
use gloo_events::EventListener;
use gloo_net::http::Request;
use state::*;
use std::rc::Rc;
use ui::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlSelectElement;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Clone, Debug, PartialEq, Routable)]
pub enum Route {
    #[at("/")]
    Catalog,
    #[at("/creature/:id")]
    Creature { id: String },
    #[at("/collection")]
    Collection,
    #[at("/compare")]
    Compare,
    #[at("/packs")]
    Packs,
    #[at("/profiles")]
    Profiles,
    #[at("/transfer")]
    Transfer,
    #[at("/settings")]
    Settings,
    #[not_found]
    #[at("/404")]
    NotFound,
}

fn switch(route: Route) -> Html {
    match route {
        Route::Catalog => html! {<catalog::CatalogView/>},
        Route::Creature { id } => html! {<catalog::CreatureView id={id}/>},
        Route::Collection => html! {<collection::CollectionView/>},
        Route::Compare => html! {<collection::CompareView/>},
        Route::Packs => html! {<settings::PacksView/>},
        Route::Profiles => html! {<settings::ProfilesView/>},
        Route::Transfer => html! {<transfer::TransferView/>},
        Route::Settings => html! {<settings::SettingsView/>},
        Route::NotFound => {
            html! {<div class="empty-state"><h1>{"Здесь пока нет тропинки"}</h1><Link<Route> to={Route::Catalog}>{"Вернуться в справочник"}</Link<Route>></div>}
        }
    }
}

#[component]
fn Shell() -> Html {
    let ctx = use_app();
    let route = use_route::<Route>().unwrap_or(Route::Catalog);
    let menu = use_state(|| false);
    let change_profile = {
        let ctx = ctx.clone();
        Callback::from(move |e: Event| {
            let mut next = (*ctx.data).clone();
            next.preferences.active_profile_id =
                e.target_unchecked_into::<HtmlSelectElement>().value();
            ctx.save(next, "Профиль выбран", Callback::noop());
        })
    };
    let profile = ctx.data.profile();
    {
        let menu = menu.clone();
        use_effect_with(route.clone(), move |_| {
            menu.set(false);
            || ()
        });
    }
    let current = match route {
        Route::Catalog | Route::Creature { .. } => "Каталог существ",
        Route::Collection => "Моя коллекция",
        Route::Compare => "Сравнение",
        Route::Packs => "Пакеты существ",
        Route::Profiles => "Профили",
        Route::Transfer => "Импорт и экспорт",
        _ => "Настройки",
    };
    let game = profile.game;
    html! {<div class="app-shell"><aside id="main-navigation" class={classes!("sidebar",(*menu).then_some("menu-open"))}><Link<Route> classes="brand" to={Route::Catalog}><span class="brand-mark"><Icon name="box"/></span><div>{"SATAYA"}<small>{"ARK FIELD GUIDE"}</small></div></Link<Route>><div class="sidebar-label">{"ИССЛЕДОВАТЬ"}</div><nav>{for[(Route::Catalog,"grid","Каталог существ"),(Route::Collection,"book","Моя коллекция"),(Route::Compare,"compare","Сравнение")].iter().map(|(r,icon,label)|{
        let active=route==*r||(*r==Route::Catalog&&matches!(route,Route::Creature{..}));html!{<Link<Route> classes={classes!("nav-item",active.then_some("active"))} to={r.clone()}><Icon name={*icon}/><span>{label}</span>if *r==Route::Collection{<span class="nav-count">{ctx.data.creatures.iter().filter(|c|c.profile_id==profile.id).count()}</span>}</Link<Route>>}
    })}</nav><div class="sidebar-label">{"ОРГАНИЗОВАТЬ"}</div><nav>{for[(Route::Packs,"box","Пакеты существ"),(Route::Profiles,"globe","Миры и серверы"),(Route::Transfer,"transfer","Импорт и экспорт"),(Route::Settings,"settings","Настройки")].iter().map(|(r,icon,label)|{html!{<Link<Route> classes={classes!("nav-item",(route==*r).then_some("active"))} to={r.clone()}><Icon name={*icon}/><span>{label}</span></Link<Route>>}})}</nav><div class="sidebar-bottom"><div class="field-note"><span class="status-dot"/><strong>{"Личная коллекция"}</strong><p>{"Ваши дино и заметки хранятся в этом браузере."}</p></div><div class="sidebar-profile"><span class="avatar">{"S"}</span><div><strong>{&profile.name}</strong><small>{game.label()}</small></div></div><span class="version-label">{"FIELD GUIDE / V.01"}</span></div></aside>
    <div class="main-shell"><header class="topbar"><button class="icon-button mobile-menu" aria-label={if *menu{"Закрыть меню"}else{"Открыть меню"}} aria-expanded={menu.to_string()} aria-controls="main-navigation" onclick={let menu=menu.clone();Callback::from(move |_|menu.set(!*menu))}><Icon name="menu"/></button><div class="breadcrumb"><span>{"Справочник"}</span><Icon name="chevron"/><strong>{current}</strong></div><div class="topbar-actions"><div class="edition-switch">{for[Game::Asa,Game::Ase].map(|g|{let ctx=ctx.clone();html!{<button class={classes!((g==game).then_some("active"))} disabled={*ctx.busy||!*ctx.ready} onclick={Callback::from(move |_|{let mut next=(*ctx.data).clone();let id=if let Some(p)=next.profiles.iter().find(|p|p.game==g){p.id.clone()}else{let p=Profile::new(browser::new_id(),format!("Мой мир · {}",g.key().to_uppercase()),g);let id=p.id.clone();next.profiles.push(p);id};next.preferences.active_profile_id=id;ctx.save(next,"Игра выбрана",Callback::noop());})}>{g.key().to_uppercase()}</button>}})}</div><label class="profile-switch"><Icon name="globe"/><SelectField label="Текущий профиль" value={profile.id.clone()} onchange={change_profile} disabled={*ctx.busy||!*ctx.ready}>{for ctx.data.profiles.iter().map(|p|html!{<option value={p.id.clone()}>{&p.name}</option>})}</SelectField></label></div></header>
        <main class="main-content">if let Some(error)=&*ctx.storage_error{<div class="error-box storage-error" role="alert"><strong>{"Локальная БД недоступна"}</strong><p>{error}</p><p>{"Каталог доступен. Сохранение коллекции станет доступно после открытия хранилища."}</p></div>}
            <Switch<Route> render={switch}/>
        </main>
    </div>
    if let Some(target)=&*ctx.editor{<editor::Editor key={format!("{target:?}")} target={target.clone()}/>}
    if let Some(notice)=&*ctx.notice{<div class="toast" role="status"><span>{notice}</span><button class="icon-button" aria-label="Закрыть уведомление" onclick={let notice=ctx.notice.clone();Callback::from(move |_|notice.set(None))}><Icon name="close"/></button></div>}
    if *ctx.busy{<div class="saving-indicator" role="status">{"Сохраняем…"}</div>}
    </div>}
}

#[component]
fn App() -> Html {
    let data = use_state(AppData::default);
    let catalog = use_state(|| Rc::new(Catalog::default()));
    let ready = use_state(|| false);
    let busy = use_state(|| false);
    let storage_error = use_state(|| None::<String>);
    let notice = use_state(|| None::<String>);
    let editor = use_state(|| None::<EditorTarget>);
    let compare = use_state(Vec::<String>::new);
    let catalog_error = use_state(|| None::<String>);
    {
        let data = data.clone();
        let ready = ready.clone();
        let storage_error = storage_error.clone();
        let catalog = catalog.clone();
        let catalog_error = catalog_error.clone();
        let notice = notice.clone();
        use_effect_with((), move |_| {
            browser::initialize_browser();
            let loaded = data.clone();
            spawn_local(async move {
                match storage::load().await {
                    Ok(value) => {
                        browser::apply_theme(&value.preferences.theme);
                        loaded.set(value);
                        ready.set(true);
                    }
                    Err(error) => storage_error.set(Some(error)),
                }
            });
            spawn_local(async move {
                let response = async {
                    let response = Request::get(&browser::asset_url("data/catalog.json"))
                        .send()
                        .await
                        .map_err(|e| e.to_string())?;
                    if !response.ok() {
                        return Err(format!("HTTP {}", response.status()));
                    }
                    let catalog = response
                        .json::<Catalog>()
                        .await
                        .map_err(|e| e.to_string())?;
                    if catalog.version != 1 || catalog.creatures.is_empty() {
                        return Err("Неподдерживаемый формат справочника".into());
                    }
                    Ok(catalog)
                }
                .await;
                match response {
                    Ok(value) => catalog.set(Rc::new(value)),
                    Err(error) => catalog_error.set(Some(error)),
                }
            });
            let event =
                EventListener::new(&web_sys::window().expect("window"), "ark-sync", move |_| {
                    let data = data.clone();
                    let notice = notice.clone();
                    spawn_local(async move {
                        match storage::load().await {
                            Ok(value) => {
                                browser::apply_theme(&value.preferences.theme);
                                data.set(value);
                                notice.set(Some("Коллекция обновлена из другой вкладки".into()));
                            }
                            Err(error) => notice.set(Some(error)),
                        }
                    });
                });
            move || drop(event)
        });
    }
    let context = AppContext {
        data,
        catalog: (*catalog).clone(),
        ready,
        busy,
        storage_error,
        notice,
        editor,
        compare,
    };
    html! {<ContextProvider<AppContext> context={context}><HashRouter>if catalog.creatures.is_empty(){<div class="app-loading"><span class="brand-mark"><Icon name="box"/></span><h1>{"SATAYA"}</h1>if let Some(error)=&*catalog_error{<p class="error-text">{format!("Не удалось открыть справочник: {error}")}</p>}else{<p>{"Открываем полевой справочник…"}</p>}</div>}else{<Shell/>}</HashRouter></ContextProvider<AppContext>>}
}

fn main() {
    console_error_panic_hook::set_once();
    if let Some(element) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("loading"))
    {
        element.remove();
    }
    yew::Renderer::<App>::new().render();
}
