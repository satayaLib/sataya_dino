use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/browser.js")]
extern "C" {
    #[wasm_bindgen(js_name = newId)]
    pub fn new_id() -> String;
    pub fn now() -> String;
    #[wasm_bindgen(js_name = assetUrl)]
    pub fn asset_url(path: &str) -> String;
    #[wasm_bindgen(js_name = databaseName)]
    pub fn database_name() -> String;
    pub fn download(content: &str, name: &str, mime: &str);
    #[wasm_bindgen(js_name = applyTheme)]
    pub fn apply_theme(theme: &str);
    #[wasm_bindgen(js_name = notifyChange)]
    pub fn notify_change();
    #[wasm_bindgen(js_name = initializeBrowser)]
    pub fn initialize_browser();
    #[wasm_bindgen(js_name = updateAvailable)]
    pub fn update_available() -> bool;
    #[wasm_bindgen(catch, js_name = activateUpdate)]
    pub async fn activate_update() -> Result<(), JsValue>;
    #[wasm_bindgen(catch, js_name = readText)]
    pub async fn read_text(file: JsValue) -> Result<String, JsValue>;
    #[wasm_bindgen(catch, js_name = loadImage)]
    pub async fn load_image(file: JsValue) -> Result<String, JsValue>;
    #[wasm_bindgen(js_name = releaseImage)]
    pub fn release_image(url: &str);
    #[wasm_bindgen(catch, js_name = recognizeImage)]
    pub async fn recognize_image(
        url: &str,
        crop: &str,
        lang: &str,
        progress: &js_sys::Function,
    ) -> Result<String, JsValue>;
    #[wasm_bindgen(js_name = cancelOcr)]
    pub fn cancel_ocr();
    #[wasm_bindgen(catch, js_name = prepareOffline)]
    pub async fn prepare_offline(progress: &js_sys::Function) -> Result<String, JsValue>;
    #[wasm_bindgen(catch, js_name = storageInfo)]
    pub async fn storage_info() -> Result<String, JsValue>;
    #[wasm_bindgen(catch, js_name = requestPersistence)]
    pub async fn request_persistence() -> Result<bool, JsValue>;
}

pub fn error(value: JsValue) -> String {
    value
        .as_string()
        .or_else(|| {
            js_sys::Reflect::get(&value, &"message".into())
                .ok()
                .and_then(|v| v.as_string())
        })
        .unwrap_or_else(|| "Не удалось выполнить операцию".into())
}
