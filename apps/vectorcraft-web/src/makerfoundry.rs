//! Opening a design from a MakerFoundry link (PF-12): the page opened as
//! `…/patternfoundry/?open=<URL-encoded design file address>` (MakerFoundry's "Open in
//! PatternFoundry" button) downloads that file and opens it as if it had been dropped on the page.
//!
//! The address is read once and taken out of the page address, so reloading or bookmarking the
//! page doesn't open it again. Only an address [`pf_ai::makerfoundry::open_address`] accepts is
//! fetched, and the address fetched is the one it rebuilt from the checked parts; the fetch sends
//! no cookies, follows no redirect, stops after [`TIME_LIMIT_MS`] and reads at most
//! [`MAX_DOWNLOAD`] bytes. Why a file didn't open goes to the status bar, MakerFoundry's own
//! sentence when it gave one.

use std::cell::RefCell;
use std::rc::Rc;

use pf_ai::makerfoundry::{MAX_DOWNLOAD, OpenError, download_name, download_refusal, open_address, open_param, without_open};
use wasm_bindgen::JsCast as _;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen_futures::JsFuture;

use crate::web::Inbox;

/// How long a download may take before it is stopped.
const TIME_LIMIT_MS: i32 = 60_000;

/// Sentences for the status bar, waiting for the next frame.
pub(crate) type Notes = Rc<RefCell<Vec<String>>>;

/// The tab's session-storage key remembering the staging switch (this tab only, until it closes).
const STAGING_KEY: &str = "pf-services";

/// Choose the services for this tab: `?services=staging` (exactly that) or an earlier switch in
/// this tab's session storage makes the app use the compiled-in staging services, with a banner;
/// anything else leaves production. Run before the `open` link is read.
pub(crate) fn choose_services() {
    let Some(window) = web_sys::window() else { return };
    let storage = window.session_storage().ok().flatten();
    let remembered = storage.as_ref().and_then(|s| s.get_item(STAGING_KEY).ok().flatten()).as_deref() == Some("staging");
    let search = window.location().search().unwrap_or_default();
    if !pf_ai::makerfoundry::wants_staging(&search, remembered) {
        return;
    }
    pf_ai::makerfoundry::set_staging(true);
    if let Some(s) = storage {
        // (not kept: the next load of this tab needs the switch again)
        let _ = s.set_item(STAGING_KEY, "staging");
    }
    show_banner(&window);
}

/// The visible banner, a status region at the top of the page.
fn show_banner(window: &web_sys::Window) {
    let Some(document) = window.document() else { return };
    let (Ok(el), Some(body)) = (document.create_element("div"), document.body()) else { return };
    el.set_id("vectorcraft_staging");
    // Best effort: the staging switch still works without the banner.
    el.set_attribute("role", "status").ok();
    el.set_attribute(
        "style",
        "position:fixed;top:0;left:0;right:0;z-index:10;background:#b45309;color:#fff;font:600 13px system-ui,sans-serif;text-align:center;padding:2px 0;pointer-events:none",
    )
    .ok();
    el.set_text_content(Some(pf_ai::makerfoundry::STAGING_BANNER));
    if let Err(e) = body.append_child(&el) {
        log::warn!("couldn't show the staging banner: {e:?}");
    }
}

/// The `open` link in the page address, taken out of it (the rest of the address is kept).
pub(crate) fn take_link() -> Option<String> {
    let window = web_sys::window()?;
    let location = window.location();
    let search = location.search().ok()?;
    let link = open_param(&search);
    if search.split(['?', '&']).any(|kv| kv == "open" || kv.starts_with("open=")) {
        let path = location.pathname().unwrap_or_default();
        let hash = location.hash().unwrap_or_default();
        let clean = format!("{path}{}{hash}", without_open(&search));
        if let Ok(history) = window.history() {
            if let Err(e) = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&clean)) {
                log::warn!("couldn't take the MakerFoundry link out of the page address: {e:?}");
            }
        }
    }
    link
}

/// Download the design `link` names and put it in `inbox` to open, or a sentence in `notes`;
/// `wake` asks for the frame that takes either in.
pub(crate) async fn open(link: String, inbox: Inbox, notes: Notes, wake: impl Fn()) {
    match download(&link).await {
        Ok((name, bytes)) => inbox.lock().unwrap_or_else(|e| e.into_inner()).push((name, bytes)),
        Err(e) => {
            log::warn!("MakerFoundry link not opened: {e}");
            notes.borrow_mut().push(e.to_string());
        }
    }
    wake();
}

async fn download(link: &str) -> Result<(String, Vec<u8>), OpenError> {
    let address = open_address(link)?;
    let net = |e: wasm_bindgen::JsValue| OpenError::Network(js_text(&e));
    let window = web_sys::window().ok_or_else(|| OpenError::Network("no window".into()))?;
    let abort = web_sys::AbortController::new().map_err(net)?;
    let init = web_sys::RequestInit::new();
    init.set_method("GET");
    init.set_credentials(web_sys::RequestCredentials::Omit);
    init.set_redirect(web_sys::RequestRedirect::Error);
    init.set_signal(Some(&abort.signal()));
    let stop = abort.clone();
    let timer = Closure::once_into_js(move || stop.abort());
    let timer_id = window.set_timeout_with_callback_and_timeout_and_arguments_0(timer.unchecked_ref(), TIME_LIMIT_MS).map_err(net)?;
    let result = fetch(&window, &address, &init).await;
    window.clear_timeout_with_handle(timer_id);
    if result.is_err() {
        abort.abort();
    }
    result
}

async fn fetch(window: &web_sys::Window, address: &str, init: &web_sys::RequestInit) -> Result<(String, Vec<u8>), OpenError> {
    let net = |e: wasm_bindgen::JsValue| OpenError::Network(js_text(&e));
    let response: web_sys::Response = JsFuture::from(window.fetch_with_str_and_init(address, init)).await.map_err(net)?.dyn_into().map_err(net)?;
    let status = response.status();
    let said = response.headers().get("content-length").ok().flatten().and_then(|n| n.trim().parse::<u64>().ok());
    if said.is_some_and(|n| n > MAX_DOWNLOAD as u64) {
        return Err(OpenError::TooLarge);
    }
    let bytes = read_capped(&response).await?;
    if !(200..300).contains(&status) {
        return Err(download_refusal(status, &String::from_utf8_lossy(&bytes)));
    }
    let disposition = response.headers().get("content-disposition").ok().flatten();
    let name = download_name(disposition.as_deref(), address)?;
    Ok((name, bytes))
}

/// The response's body, read piece by piece and given up on past [`MAX_DOWNLOAD`] bytes.
async fn read_capped(response: &web_sys::Response) -> Result<Vec<u8>, OpenError> {
    let net = |e: wasm_bindgen::JsValue| OpenError::Network(js_text(&e));
    let Some(body) = response.body() else { return Ok(Vec::new()) };
    let reader: web_sys::ReadableStreamDefaultReader = body.get_reader().dyn_into().map_err(|o: js_sys::Object| net(o.into()))?;
    let mut out = Vec::new();
    loop {
        let chunk = JsFuture::from(reader.read()).await.map_err(net)?;
        if js_sys::Reflect::get(&chunk, &"done".into()).map_err(net)?.as_bool() == Some(true) {
            return Ok(out);
        }
        let value: js_sys::Uint8Array = js_sys::Reflect::get(&chunk, &"value".into()).map_err(net)?.dyn_into().map_err(net)?;
        if out.len() + value.length() as usize > MAX_DOWNLOAD {
            // Stop the download rather than read the rest.
            let _ = reader.cancel();
            return Err(OpenError::TooLarge);
        }
        let start = out.len();
        out.resize(start + value.length() as usize, 0);
        value.copy_to(&mut out[start..]);
    }
}

/// A browser error as a short sentence ("TypeError: Failed to fetch").
fn js_text(e: &wasm_bindgen::JsValue) -> String {
    if let Some(err) = e.dyn_ref::<js_sys::Error>() {
        return format!("{}: {}", String::from(err.name()), String::from(err.message()));
    }
    e.as_string().unwrap_or_else(|| "the browser refused the download".into())
}
