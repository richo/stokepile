use hex;
use web_sys;
use web_sys::{Request, RequestInit, RequestMode, Response, Element};

mod utils;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use stokepile_shared::staging::{UploadDescriptor, GroupByDevice};
use uuid::Uuid;

static BASE_URL: &'static str = "http://localhost:8000";

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
        for (device, media) in media.grouped_by_device() {
            let val = document.create_element("li").expect("Create element");
            val.set_inner_html(device);
            val.set_class_name("pure-menu-item pure-menu-heading");
            media_list.append_child(&val).expect("append");

            for file in media {
                let val = document.create_element("li").expect("Create element");
                val.set_inner_html(&file.name());
                val.set_class_name("pure-menu-item media-list-item");
                val.set_attribute("data-content-hash", &hex::encode(&file.content_hash)).unwrap();
                val.set_attribute("data-uuid", file.uuid.to_hyphenated().encode_lower(&mut Uuid::encode_buffer())).unwrap();
                media_list.append_child(&val).expect("append");
            }
        }
    }
}

#[wasm_bindgen]
pub async fn clear_staged_media() {
    clear_element_children(&media_list())
}

#[wasm_bindgen]
pub async fn activate_media(uuid: String) {
    // TODO(richo) cache this
    let media = fetch_staged_media().await.expect("fetch staged media");
    // TODO(richo) is it worth sending this representation on the wire?
    let parsed = Uuid::parse_str(&uuid).expect("parse uuid");

    let document = document();
    let player = document.get_element_by_id("media-player")
        .expect("couldn't find media player");
    clear_element_children(&player);

    let desc = media.iter()
        .filter(|desc| desc.uuid == parsed)
        .next()
        .expect(&format!("Couldn't find media with uuid: {}", parsed));

    let name_field = document.get_element_by_id("name")
        .expect("couldn't find media player");
    // TODO(richo) This doesn't make any sense rn
    name_field.set_attribute("value", &desc.device_name)
        .expect("setAttribute");

    let source = document.create_element("source")
        .expect("create source");
    source.set_attribute("src", &format!("{}/api/media/{}", BASE_URL, uuid))
        .unwrap();

    player.append_child(&source)
        .unwrap();
}

fn clear_element_children(el: &Element) {
    let children = el.children();
    for _ in 0..children.length() {
        children.item(0).map(|x| x.remove());
    }
}

fn media_list() -> Element {
    let document = document();
    document.get_element_by_id("media-list").expect("document should have a body")
}

pub async fn fetch_staged_media() -> Result<Vec<UploadDescriptor>, JsValue> {
    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);

    // TODO(richo) pull the window object apart to figure out where we are
    let url = format!("{}/api/media", BASE_URL);

    let request = Request::new_with_str_and_init(&url, &opts)?;

    // request
    //     .headers()
    //     .set("Accept", "application/vnd.github.v3+json")?;

    let window = window();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;

    // `resp_value` is a `Response` object.
    assert!(resp_value.is_instance_of::<Response>());
    let resp: Response = resp_value.dyn_into().unwrap();

    // Convert this other `Promise` into a rust `Future`.
    let json = JsFuture::from(resp.json()?).await?;

    Ok(json.into_serde().expect("Couldn't parse json"))
}
