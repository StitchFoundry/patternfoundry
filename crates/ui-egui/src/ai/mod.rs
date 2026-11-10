//! PatternFoundry's AI help (PF-8): Ask About Patterns ([`ask`]), Check My Muslin ([`muslin`]) and
//! the Foundry Account that can pay for them ([`account`]). Requests go out through the host's `http_post` service (desktop, on a
//! thread of their own) or `http_post_async` (the web, the browser's fetch), always through the
//! Foundry Account (a session paid in credits); the windows pick the answers up on a later frame.
//!
//! Adapted from StitchFoundry's AI windows (`crates/ui-egui/src/embroidery/{ask,account,prompt}.rs`
//! in github.com/Stitchwright/StitchFoundry), as `docs/start-here.md` plans.

use std::sync::mpsc::{Receiver, channel};

use pf_ai::{AiError, HttpRequest, cloud};

use crate::VectorcraftApp;

pub mod account;
pub mod ask;
pub mod muslin;
pub mod publish;

/// The secret an older version kept an Anthropic API key as. Never read for use: found on
/// load, it is deleted ([`account::load`]).
pub(crate) const LEGACY_KEY_NAME: &str = "anthropic-api-key";
/// The secret an older version kept the "pay with credits" choice as (credits are the only way now).
pub(crate) const LEGACY_CREDITS_NAME: &str = "foundry-pay-with-credits";

/// The AI windows between frames. Not saved: sessions are kept by the host's secret store.
#[derive(Default)]
pub struct AiState {
    /// The time of the last frame drawn (egui time, seconds).
    pub now: f64,
    pub ask: ask::AskState,
    pub account: account::AccountState,
    pub muslin: muslin::MuslinState,
    pub publish: publish::PublishState,
}

/// Whether this build can send AI requests.
pub fn available(app: &VectorcraftApp) -> bool {
    app.services.http_post.is_some() || app.services.http_post_async.is_some()
}

/// Sends AI request `http` (built with [`account::key_to_send`], the session token): to `path` on
/// the Foundry service with `extra` headers, its refusals read as the service's (sign in,
/// credits). Nothing goes to Anthropic directly.
pub(crate) fn post_ai<T: Send + 'static>(
    app: &VectorcraftApp,
    http: HttpRequest,
    thread: &str,
    path: &str,
    extra: Vec<(&'static str, String)>,
    parse: fn(u16, &str) -> Result<T, AiError>,
) -> Result<Receiver<Result<T, AiError>>, String> {
    account::require_account(app)?;
    let server = account::server(app).ok_or_else(|| AiError::NoServer.to_string())?;
    let http = cloud::through_cloud(http, &server, path, extra).map_err(|e| e.to_string())?;
    post_plain(app, http, thread, move |status, body| {
        if matches!(status, 401 | 402 | 403 | 503) { Err(cloud::status_error(status, body)) } else { parse(status, body) }
    })
}

/// Sends `http` with the host's service and reads the answer with `parse`: on the desktop on a
/// thread named `thread`, on the web through the browser. The result arrives on the receiver;
/// nothing waits for it once the receiver is dropped.
pub(crate) fn post_plain<T: Send + 'static, F: Fn(u16, &str) -> Result<T, AiError> + Send + 'static>(
    app: &VectorcraftApp,
    http: HttpRequest,
    thread: &str,
    parse: F,
) -> Result<Receiver<Result<T, AiError>>, String> {
    let (tx, rx) = channel();
    if let Some(post) = app.services.http_post.clone() {
        std::thread::Builder::new()
            .name(thread.into())
            .spawn(move || {
                let r = vectorcraft_engine::guard::catch_panic(|| post(&http))
                    .unwrap_or_else(|msg| Err(format!("internal error: {msg} (please report this bug)")))
                    .map_err(AiError::Network)
                    .and_then(|(status, body)| parse(status, &body));
                let _ = tx.send(r); // (nothing may wait any more)
            })
            .map_err(|e| format!("can't send the request: {e}"))?;
    } else if let Some(post) = app.services.http_post_async.clone() {
        post(
            http,
            Box::new(move |r| {
                let _ = tx.send(r.map_err(AiError::Network).and_then(|(status, body)| parse(status, &body))); // (nothing may wait any more)
            }),
        );
    } else {
        return Err("this needs a connection to the AI service, which this build doesn't have".into());
    }
    Ok(rx)
}

/// Each frame: take in the answers that arrived and draw the open windows.
pub fn frame(app: &mut VectorcraftApp, ctx: &egui::Context) {
    app.ai.now = ctx.input(|i| i.time);
    ask::poll(app, ctx);
    account::poll(app, ctx);
    publish::poll(app, ctx);
    ask::window(app, ctx);
    account::window(app, ctx);
    muslin::window(app, ctx);
    publish::window(app, ctx);
}

#[cfg(test)]
mod tests;
