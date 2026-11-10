use ark_core::*;
use rexie::{Index, ObjectStore, Rexie, TransactionMode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use wasm_bindgen::JsValue;

const STORES: &[&str] = &["owned_creatures", "profiles", "preferences", "wishlist"];

#[derive(Deserialize, Serialize)]
struct Meta {
    id: String,
    schema_version: u32,
    revision: u64,
    active_profile_id: String,
    theme: String,
    last_export_at: Option<String>,
}
#[derive(Deserialize, Serialize)]
struct Bookmark {
    id: String,
    kind: String,
    creature_id: String,
}

fn js<T: Serialize>(v: &T) -> Result<JsValue, String> {
    js_sys::JSON::parse(&serde_json::to_string(v).map_err(|e| e.to_string())?)
        .map_err(crate::browser::error)
}
fn decode<T: serde::de::DeserializeOwned>(v: JsValue) -> Result<T, String> {
    serde_wasm_bindgen::from_value(v).map_err(|e| e.to_string())
}
fn db_error(e: rexie::Error) -> String {
    format!("Не удалось сохранить данные в браузере: {e}")
}

async fn open() -> Result<Rexie, String> {
    Rexie::builder(&crate::browser::database_name())
        .version(1)
        .add_object_store(
            ObjectStore::new("owned_creatures")
                .key_path("id")
                .add_index(Index::new("profile_id", "profile_id"))
                .add_index(Index::new("creature_id", "creature_id")),
        )
        .add_object_store(ObjectStore::new("profiles").key_path("id"))
        .add_object_store(ObjectStore::new("preferences").key_path("id"))
        .add_object_store(ObjectStore::new("wishlist").key_path("id"))
        .build()
        .await
        .map_err(db_error)
}

pub async fn load() -> Result<AppData, String> {
    let db = open().await?;
    let tx = db
        .transaction(STORES, TransactionMode::ReadOnly)
        .map_err(db_error)?;
    let meta = tx
        .store("preferences")
        .map_err(db_error)?
        .get("state".into())
        .await
        .map_err(db_error)?;
    let Some(meta) = meta else {
        tx.done().await.map_err(db_error)?;
        return Ok(AppData::default());
    };
    let meta: Meta = decode(meta)?;
    let profiles = tx
        .store("profiles")
        .map_err(db_error)?
        .get_all(None, None)
        .await
        .map_err(db_error)?
        .into_iter()
        .map(decode)
        .collect::<Result<Vec<_>, _>>()?;
    let creatures = tx
        .store("owned_creatures")
        .map_err(db_error)?
        .get_all(None, None)
        .await
        .map_err(db_error)?
        .into_iter()
        .map(decode)
        .collect::<Result<Vec<_>, _>>()?;
    let bookmarks = tx
        .store("wishlist")
        .map_err(db_error)?
        .get_all(None, None)
        .await
        .map_err(db_error)?
        .into_iter()
        .map(decode::<Bookmark>)
        .collect::<Result<Vec<_>, _>>()?;
    tx.done().await.map_err(db_error)?;
    let mut preferences = Preferences {
        active_profile_id: meta.active_profile_id,
        theme: meta.theme,
        last_export_at: meta.last_export_at,
        favorites: Default::default(),
        wishlist: Default::default(),
    };
    for b in bookmarks {
        if b.kind == "favorite" {
            preferences.favorites.insert(b.creature_id);
        } else {
            preferences.wishlist.insert(b.creature_id);
        }
    }
    let data = AppData {
        schema_version: meta.schema_version,
        revision: meta.revision,
        profiles,
        creatures,
        preferences,
    };
    data.validate()?;
    Ok(data)
}

fn bookmarks(data: &AppData) -> BTreeMap<String, Bookmark> {
    data.preferences
        .favorites
        .iter()
        .map(|id| ("favorite", id))
        .chain(data.preferences.wishlist.iter().map(|id| ("wish", id)))
        .map(|(kind, id)| {
            let key = format!("{kind}:{id}");
            (
                key.clone(),
                Bookmark {
                    id: key,
                    kind: kind.into(),
                    creature_id: id.clone(),
                },
            )
        })
        .collect()
}

pub async fn save(before: &AppData, mut after: AppData) -> Result<AppData, String> {
    after.validate()?;
    let db = open().await?;
    let tx = db
        .transaction(STORES, TransactionMode::ReadWrite)
        .map_err(db_error)?;
    let prefs = tx.store("preferences").map_err(db_error)?;
    let stored = prefs.get("state".into()).await.map_err(db_error)?;
    let revision = stored
        .map(decode::<Meta>)
        .transpose()?
        .map_or(0, |m| m.revision);
    if revision != before.revision {
        return Err("Коллекция изменилась в другой вкладке. Обновите данные и повторите сохранение; ваш черновик оставлен открытым.".into());
    }
    after.revision = revision + 1;
    let store = tx.store("owned_creatures").map_err(db_error)?;
    let old: BTreeMap<_, _> = before.creatures.iter().map(|c| (&c.id, c)).collect();
    let new: BTreeMap<_, _> = after.creatures.iter().map(|c| (&c.id, c)).collect();
    for id in old.keys().filter(|id| !new.contains_key(*id)) {
        store
            .delete(JsValue::from_str(id))
            .await
            .map_err(db_error)?;
    }
    for (id, c) in &new {
        if old.get(id).is_none_or(|old| old != c) {
            store.put(&js(*c)?, None).await.map_err(db_error)?;
        }
    }
    let profiles = tx.store("profiles").map_err(db_error)?;
    for p in &before.profiles {
        if !after.profiles.iter().any(|x| x.id == p.id) {
            profiles
                .delete(JsValue::from_str(&p.id))
                .await
                .map_err(db_error)?;
        }
    }
    for p in &after.profiles {
        if !before.profiles.contains(p) || revision == 0 {
            profiles.put(&js(p)?, None).await.map_err(db_error)?;
        }
    }
    let wishes = tx.store("wishlist").map_err(db_error)?;
    let old = bookmarks(before);
    let new = bookmarks(&after);
    for id in old.keys().filter(|id| !new.contains_key(*id)) {
        wishes
            .delete(JsValue::from_str(id))
            .await
            .map_err(db_error)?;
    }
    for (id, b) in &new {
        if !old.contains_key(id) {
            wishes.put(&js(b)?, None).await.map_err(db_error)?;
        }
    }
    prefs
        .put(
            &js(&Meta {
                id: "state".into(),
                schema_version: 1,
                revision: after.revision,
                active_profile_id: after.preferences.active_profile_id.clone(),
                theme: after.preferences.theme.clone(),
                last_export_at: after.preferences.last_export_at.clone(),
            })?,
            None,
        )
        .await
        .map_err(db_error)?;
    tx.done().await.map_err(db_error)?;
    Ok(after)
}
