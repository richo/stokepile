use web_sys;
use web_sys::{Request, RequestInit, RequestMode, Response, Element};

mod utils;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use stokepile_shared::staging::StagedFile;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

fn document() -> web_sys::Document {
    window().document().expect("should have a document on window")
}

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    utils::set_panic_hook();
    Ok(())
}

#[wasm_bindgen]
extern {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub async fn load_staged_media() {
    let document = document();
    let media_list = media_list();
    clear_staged_media().await;
    if let Ok(media) = fetch_staged_media().await {
        for file in media {
            let val = document.create_element("li").expect("Create element");
            val.set_inner_html(file.content_path.to_str().expect("couldn't convert patht o string"));
            media_list.append_child(&val).expect("append");
        }
    }
}

#[wasm_bindgen]
pub async fn clear_staged_media() {
    let children = media_list().children();
    for i in 0..=children.length() {
        children.item(i).map(|x| x.remove());
    }
}

fn media_list() -> Element {
    let document = document();
    document.get_element_by_id("media-list").expect("document should have a body")
}

pub async fn fetch_staged_media() -> Result<Vec<StagedFile>, JsValue> {
    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);

    // TODO(richo) pull the window object apart to figure out where we are
    let url = "http://localhost:8000/api/media";

    let request = Request::new_with_str_and_init(&url, &opts)?;

    // request
    //     .headers()
    //     .set("Accept", "application/vnd.github.v3+json")?;

    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;

    // `resp_value` is a `Response` object.
    assert!(resp_value.is_instance_of::<Response>());
    let resp: Response = resp_value.dyn_into().unwrap();

    // Convert this other `Promise` into a rust `Future`.
    let json = JsFuture::from(resp.json()?).await?;

    Ok(json.into_serde().expect("Couldn't parse json"))
}
