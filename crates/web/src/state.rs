use ark_core::*;
use serde::Deserialize;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct Catalog {
    pub version: u32,
    pub packs: Vec<ContentPack>,
    pub creatures: Vec<CreatureDefinition>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EditorTarget {
    New(Option<String>, bool),
    Edit(String),
    Copy(String),
}

#[derive(Clone, PartialEq)]
pub struct AppContext {
    pub data: UseStateHandle<AppData>,
    pub catalog: Rc<Catalog>,
    pub ready: UseStateHandle<bool>,
    pub busy: UseStateHandle<bool>,
    pub storage_error: UseStateHandle<Option<String>>,
    pub notice: UseStateHandle<Option<String>>,
    pub editor: UseStateHandle<Option<EditorTarget>>,
    pub compare: UseStateHandle<Vec<String>>,
}

impl AppContext {
    pub fn save(&self, next: AppData, message: &str, done: Callback<()>) {
        if !*self.ready || *self.busy {
            return;
        }
        let before = (*self.data).clone();
        let ctx = self.clone();
        let message = message.to_owned();
        self.busy.set(true);
        spawn_local(async move {
            match crate::storage::save(&before, next).await {
                Ok(data) => {
                    crate::browser::apply_theme(&data.preferences.theme);
                    ctx.data.set(data);
                    ctx.notice.set(Some(message));
                    crate::browser::notify_change();
                    done.emit(());
                }
                Err(error) => ctx.notice.set(Some(error)),
            }
            ctx.busy.set(false);
        });
    }
    pub fn add(&self, id: Option<String>, screenshot: bool) {
        self.editor.set(Some(EditorTarget::New(id, screenshot)));
    }
    pub fn bookmark(&self, id: String, wish: bool) {
        let mut next = (*self.data).clone();
        let items = if wish {
            &mut next.preferences.wishlist
        } else {
            &mut next.preferences.favorites
        };
        if !items.remove(&id) {
            items.insert(id);
        }
        self.save(next, "Сохранено", Callback::noop());
    }
    pub fn export_csv(&self, records: Vec<OwnedCreature>) {
        match ark_core::export_csv(&self.data, &records) {
            Ok(text) => {
                crate::browser::download(&text, "sataya-collection.csv", "text/csv;charset=utf-8");
                let mut next = (*self.data).clone();
                next.preferences.last_export_at = Some(crate::browser::now());
                self.save(next, "CSV выгружен", Callback::noop());
            }
            Err(error) => self.notice.set(Some(error)),
        }
    }
    pub fn export_json(&self) {
        match export_backup(&self.data, &crate::browser::now()) {
            Ok(text) => crate::browser::download(
                &text,
                "sataya-backup.json",
                "application/json;charset=utf-8",
            ),
            Err(error) => self.notice.set(Some(error)),
        }
    }
}

#[hook]
pub fn use_app() -> AppContext {
    use_context::<AppContext>().expect("app context")
}
