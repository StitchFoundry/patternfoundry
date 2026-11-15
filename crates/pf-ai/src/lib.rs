//! # PatternFoundry AI
//!
//! What PatternFoundry asks an AI model for, and how its replies are read. The model is Claude,
//! reached through the Foundry cloud service, paid in Foundry credits ([`cloud`]): the Foundry
//! Account is the only way to sign in and pay for AI in the app. No network code lives here: the app sends the
//! [`HttpRequest`]s built here with its own client and hands the answers back to be read.
//!
//! * [`ask`] – Ask About Patterns: questions about sewing patterns and PatternFoundry, answered
//!   by Claude, which can look at the pattern open now and, with the user's leave, change it
//!   through PatternFoundry's commands.
//! * [`muslin`] – Check My Muslin: photos of a muslin (a test garment) tried on, looked at by
//!   Claude for fit, which says what to alter and, with the user's leave, alters the pattern.
//! * [`cloud`] – Foundry accounts: signing in, the credits, and AI requests sent through the
//!   service.
//!
//! Adapted from StitchFoundry's AI client (`crates/sf-core/src/ai.rs` in
//! github.com/Stitchwright/StitchFoundry, the same owner and licence), as `docs/start-here.md`
//! plans.
//!
//! Body measurements are personal data: nothing here sends them by itself; a request carries
//! only what the app puts in it.

use serde_json::Value;

pub mod ask;
pub mod cloud;
pub mod makerfoundry;
pub mod muslin;

/// The Messages API endpoint.
pub const API_URL: &str = "https://api.anthropic.com/v1/messages";
/// The API version the requests are written against.
pub const API_VERSION: &str = "2023-06-01";
/// The model that answers (the one the Foundry service allows).
pub const MODEL: &str = "claude-opus-5-5";
/// The beta that lets a request the model declines run again on the model Anthropic recommends
/// (`"fallbacks": "default"`).
pub const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

/// The longest side, in pixels, a photo is sent at (the model sees no more detail).
pub const IMAGE_SIDE: u32 = 1568;
/// The largest image sent, as base64 (the API takes up to 5 MB an image).
pub const MAX_IMAGE_BASE64: usize = 5_000_000;

/// An image to send: PNG or JPEG bytes in base64.
#[derive(Clone, Debug, PartialEq)]
pub struct AiImage {
    /// `image/png` or `image/jpeg`.
    pub media_type: &'static str,
    pub data_base64: String,
}

/// An HTTP POST for the app's client to send.
#[derive(Clone, Debug, PartialEq)]
pub struct HttpRequest {
    pub url: String,
    pub headers: Vec<(&'static str, String)>,
    pub body: String,
}

/// Why no answer came back. The messages are for the user; the UI shows them translated.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum AiError {
    #[error("AI now uses your Foundry Account. Sign in to continue.")]
    NoKey,
    #[error("The API key isn't valid: paste it again from the Anthropic Console.")]
    BadKey,
    #[error("Type a question first.")]
    NoQuestion,
    #[error("The question is too long: keep it under 2,000 characters.")]
    LongQuestion,
    /// Check My Muslin was asked with no photo.
    #[error("Add a photo of the muslin first.")]
    NoPhoto,
    #[error("The image is too large to send: crop it, or send a smaller copy.")]
    LargeImage,
    /// The key was refused (401) or may not use the model (403).
    #[error("Anthropic refused the API key: {0}")]
    Key(String),
    /// The account can't pay for the request (402).
    #[error("Your Anthropic account can't pay for this: {0}")]
    Billing(String),
    /// Too many requests (429) or the service is overloaded (529, 503).
    #[error("The AI service is busy: try again in a minute.")]
    Busy,
    /// The service failed (5xx).
    #[error("The AI service failed (error {0}): try again later.")]
    Service(u16),
    /// Any other refusal of the request, with the API's message.
    #[error("The request failed (error {0}): {1}")]
    Request(u16, String),
    /// The model declined to answer.
    #[error("The AI declined to answer this: ask another way.")]
    RefusedAnswer,
    /// The reply holds no answer, or ran out of room before one.
    #[error("The reply had no answer: try again, or ask something narrower.")]
    NoAnswer,
    /// No Foundry service is set up in this build.
    #[error("Foundry accounts aren't open yet.")]
    NoServer,
    /// The Foundry session ended, or there is none.
    #[error("Sign in to your Foundry account (Help › Foundry Account…).")]
    SignIn,
    /// The Foundry account's email isn't confirmed yet (403), with the service's message, which
    /// says where the link went and how to have it sent again. Shown as the service wrote it.
    #[error("{0}")]
    Unconfirmed(String),
    /// The Foundry account hasn't the credits.
    #[error("Not enough Foundry credits: buy more in Foundry Account.")]
    NoCredits,
    /// An account with the email exists already.
    #[error("An account with that email already exists: sign in instead.")]
    Taken,
    /// The Foundry service can't do it (its AI or payments aren't set up).
    #[error("The Foundry service can't do this yet: try again later.")]
    CloudClosed,
    /// The request never got an answer (from the app's client).
    #[error("Couldn't reach the AI service: {0}")]
    Network(String),
}

impl AiError {
    /// Whether the same request may succeed if sent again shortly.
    pub fn retryable(&self) -> bool {
        matches!(self, Self::Busy | Self::Service(_) | Self::Network(_) | Self::NoAnswer)
    }
}

/// `key` checked for use in a header: trimmed, non-empty, printable ASCII without spaces.
pub fn clean_key(key: &str) -> Result<&str, AiError> {
    let key = key.trim();
    if key.is_empty() {
        return Err(AiError::NoKey);
    }
    if key.len() > 400 || !key.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(AiError::BadKey);
    }
    Ok(key)
}

/// The API's own message in an error body (`{"type": "error", "error": {"message": …}}`), else
/// the start of the body.
fn api_message(body: &str) -> String {
    let v: Option<Value> = serde_json::from_str(body).ok();
    if let Some(m) = v.as_ref().and_then(|v| v.pointer("/error/message")).and_then(Value::as_str) {
        return m.to_string();
    }
    let text: String = body.trim().chars().take(200).collect();
    if text.is_empty() { "no details".to_string() } else { text }
}

/// The error an HTTP status other than success stands for.
fn check_status(status: u16, body: &str) -> Result<(), AiError> {
    match status {
        200..=299 => Ok(()),
        401 | 403 => Err(AiError::Key(api_message(body))),
        402 => Err(AiError::Billing(api_message(body))),
        429 | 503 | 529 => Err(AiError::Busy),
        500..=599 => Err(AiError::Service(status)),
        _ => Err(AiError::Request(status, api_message(body))),
    }
}

/// A reply's text blocks joined (thinking left out).
fn reply_text(v: &Value) -> String {
    v.get("content")
        .and_then(Value::as_array)
        .map(|blocks| {
            blocks
                .iter()
                .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
                .filter_map(|b| b.get("text").and_then(Value::as_str))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_checked_before_they_go_in_a_header() {
        assert_eq!(clean_key("  sk-ant-1  "), Ok("sk-ant-1"));
        assert_eq!(clean_key(" "), Err(AiError::NoKey));
        assert_eq!(clean_key("sk ant"), Err(AiError::BadKey));
        assert_eq!(clean_key(&"k".repeat(401)), Err(AiError::BadKey));
    }

    #[test]
    fn statuses_and_their_errors() {
        assert_eq!(check_status(200, ""), Ok(()));
        assert_eq!(check_status(401, r#"{"error": {"message": "invalid x-api-key"}}"#), Err(AiError::Key("invalid x-api-key".into())));
        assert_eq!(check_status(402, "out of credit"), Err(AiError::Billing("out of credit".into())));
        assert_eq!(check_status(529, ""), Err(AiError::Busy));
        assert_eq!(check_status(500, ""), Err(AiError::Service(500)));
        assert_eq!(check_status(400, ""), Err(AiError::Request(400, "no details".into())));
        assert!(AiError::Busy.retryable() && !AiError::NoKey.retryable());
        let v = serde_json::json!({"content": [{"type": "thinking", "thinking": "x"}, {"type": "text", "text": "a"}, {"type": "text", "text": "b"}]});
        assert_eq!(reply_text(&v), "ab");
    }
}
