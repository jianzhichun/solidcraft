//! The product name and logo a hosting page gives this build (`window.solidcraftBrand = { name, logo }`, set before the
//! app starts; `logo` is the address of a PNG on the same origin). Without it nothing changes: SolidCraft, its own mark.

#[cfg(target_arch = "wasm32")]
use js_sys::{Reflect, Uint8Array};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;

#[cfg(target_arch = "wasm32")]
const KEY: &str = "solidcraftBrand";
#[cfg(target_arch = "wasm32")]
const LOADING_ID: &str = "solidcraft_loading";

/// What the page asked for.
#[cfg(target_arch = "wasm32")]
#[derive(Debug, PartialEq)]
pub struct Request {
    pub name: String,
    pub logo: Option<String>,
}

/// `logo` must resolve to the page's own origin: a brand never pulls an image from elsewhere.
pub fn same_origin(page: &str, logo: &str) -> Option<String> {
    let page = url::Url::parse(page).ok()?;
    let target = page.join(logo).ok()?;
    (target.origin() == page.origin() && matches!(target.scheme(), "http" | "https")).then(|| target.to_string())
}

#[cfg(target_arch = "wasm32")]
fn request(window: &web_sys::Window) -> Option<Request> {
    let obj = Reflect::get(window, &JsValue::from_str(KEY)).ok().filter(|v| v.is_object())?;
    let text = |k: &str| Reflect::get(&obj, &JsValue::from_str(k)).ok().and_then(|v| v.as_string()).filter(|s| !s.trim().is_empty());
    let name = text("name")?;
    let page = window.location().href().ok()?;
    Some(Request { name, logo: text("logo").and_then(|l| same_origin(&page, &l)) })
}

#[cfg(target_arch = "wasm32")]
async fn fetch_png(window: &web_sys::Window, address: &str) -> Option<Vec<u8>> {
    let response: web_sys::Response = JsFuture::from(window.fetch_with_str(address)).await.ok()?.dyn_into().ok()?;
    if !response.ok() {
        return None;
    }
    let buffer = JsFuture::from(response.array_buffer().ok()?).await.ok()?;
    Some(Uint8Array::new(&buffer).to_vec())
}

/// Apply the page's brand to the UI, the tab title and the loading screen. Returns whether one was given.
#[cfg(target_arch = "wasm32")]
pub async fn apply(window: &web_sys::Window, document: &web_sys::Document) -> bool {
    let Some(req) = request(window) else { return false };
    let png = match &req.logo {
        Some(address) => fetch_png(window, address).await,
        None => None,
    };
    solidcraft_ui_egui::brand::set(&req.name, png.as_deref());
    let name = solidcraft_ui_egui::brand::name();
    document.set_title(&name);
    if let Ok(Some(loading)) = document.query_selector(&format!("#{LOADING_ID} img")) {
        match (&req.logo, &png) {
            (Some(address), Some(_)) => {
                let _ = loading.set_attribute("src", address);
            }
            // The default animation is SolidCraft's own mark: a host without a logo gets no animated one.
            _ => {
                let _ = loading.set_attribute("hidden", "");
            }
        }
    }
    if let Ok(Some(label)) = document.query_selector(&format!("#{LOADING_ID} .sr-only")) {
        label.set_text_content(Some(&format!("Loading {name}\u{2026}")));
    }
    true
}

#[cfg(test)]
mod tests {
    use super::same_origin;

    #[test]
    fn a_logo_address_must_stay_on_the_pages_origin() {
        let page = "http://127.0.0.1:3350/solidz/cad/index.html";
        assert_eq!(same_origin(page, "brand/logo.png").as_deref(), Some("http://127.0.0.1:3350/solidz/cad/brand/logo.png"));
        assert_eq!(same_origin(page, "/solidz/logo.png").as_deref(), Some("http://127.0.0.1:3350/solidz/logo.png"));
        assert_eq!(same_origin(page, "https://example.org/logo.png"), None);
        assert_eq!(same_origin(page, "//example.org/logo.png"), None);
        assert_eq!(same_origin(page, "data:image/png;base64,AAAA"), None);
        assert_eq!(same_origin(page, "javascript:alert(1)"), None);
    }
}
