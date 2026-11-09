//! MakerFoundry (PF-11): publishing the document to MakerFoundry, the Foundry marketplace, as
//! a private draft the person finishes on MakerFoundry's own page. The contract is MakerFoundry's
//! `docs/api.md` ("How publishing from an app works", github.com/SiteSentrix/makerfoundry):
//!
//! - `POST https://makerfoundry.app/api/v1/designs`, `Authorization: Bearer <the person's Foundry
//!   session, sfs_…>` (never any other key), JSON `{file_name, file (base64), app}`;
//! - 201 `{id, url, kind, format}`: the draft, opened in the browser only when it is on the site
//!   the request went to; errors as `{"error": {"message"}}`, whose sentence is shown as it is.
//!
//! The document goes as SVG for now, a vector design of MakerFoundry's laser kind (its title from
//! the file name); a sewing-pattern kind (true-scale and tiled PDF, DXF-AAMA) is queued on
//! MakerFoundry's side, and [`publish`] takes any file name and contents so those can follow. As
//! elsewhere in [`super`], no network code lives here.
//!
//! Opening (PF-12): MakerFoundry's "Open in PatternFoundry" button links to the web app with
//! `?open=<design file address>`; [`open_address`] checks that address and rebuilds it,
//! [`download_name`] names the file, and [`download_refusal`] reads a refusal. The web app
//! (`apps/vectorcraft-web/src/makerfoundry.rs`) does the fetching.

use serde_json::{Value, json};

use super::HttpRequest;
use super::cloud::TOKEN_PREFIX;

/// MakerFoundry's site: where designs are sent, and the only site whose pages are opened.
pub const SITE: &str = "https://makerfoundry.app";
/// The site's host.
pub const HOST: &str = "makerfoundry.app";
/// The endpoint designs are published to.
pub const DESIGNS: &str = "/api/v1/designs";
/// The most a design file may weigh (MakerFoundry's limit for vector files).
pub const MAX_FILE: usize = 8 * 1024 * 1024;
/// The longest address or file name taken from an answer.
const MAX_ADDRESS: usize = 2048;
/// The longest sentence of MakerFoundry's shown.
const MAX_SENTENCE: usize = 300;
/// The longest title a file name keeps (MakerFoundry's slugs keep 60).
const MAX_TITLE: usize = 60;

/// Why a design wasn't published. The messages are for the user; the UI shows them translated,
/// and MakerFoundry's own sentences as they are.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum MfError {
    #[error("Sign in to your Foundry Account first: Help › Foundry Account…")]
    SignIn,
    #[error("The document is empty: there is nothing to publish.")]
    Empty,
    #[error("The design is too large for MakerFoundry: at most 8 MB.")]
    TooLarge,
    #[error("MakerFoundry's answer couldn't be read.")]
    NoAnswer,
    #[error("MakerFoundry answered with a page on another site, which wasn't opened.")]
    OffSite,
    /// MakerFoundry's own sentence (with the HTTP status it came with).
    #[error("MakerFoundry: {1}")]
    Said(u16, String),
    #[error("MakerFoundry couldn't be reached: {0}")]
    Network(String),
}

/// A published draft: its id and the page where the person finishes it.
#[derive(Clone, Debug, PartialEq)]
pub struct Published {
    pub id: u64,
    pub url: String,
}

/// The file name a design titled `title` goes as, ending in `.svg` (`ext`): MakerFoundry makes the
/// draft's title from it ("Simple_tote-bag.svg" → "Simple tote bag"). Letters and digits are
/// kept (any script), runs of anything else become one `_`, at most [`MAX_TITLE`] characters;
/// a title that leaves nothing is "PatternFoundry_design".
pub fn file_name(title: &str, ext: &str) -> String {
    let stem = title.rsplit(['/', '\\']).next().unwrap_or_default();
    let stem = match stem.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() && e.chars().all(|c| c.is_ascii_alphanumeric()) && e.len() <= 16 => s,
        _ => stem,
    };
    let mut out = String::new();
    for c in stem.chars() {
        if c.is_alphanumeric() || c == '-' {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('_') {
            out.push('_');
        }
    }
    let out: String = out.trim_matches(['_', '-']).chars().take(MAX_TITLE).collect();
    let out = out.trim_end_matches(['_', '-']);
    let stem = if out.chars().any(char::is_alphanumeric) { out } else { "PatternFoundry_design" };
    format!("{stem}.{}", ext.trim_start_matches('.'))
}

/// The request publishing `file_base64` (the file's bytes, `len` of them, base64-encoded) as
/// `file_name`, signed in with the Foundry session `token`, from app `app` ("PatternFoundry
/// 0.13.1"). Only a session token is ever sent: anything else is refused before it leaves.
pub fn publish(token: &str, file_name: &str, file_base64: &str, len: usize, app: &str) -> Result<HttpRequest, MfError> {
    let token = token.trim();
    if !token.starts_with(TOKEN_PREFIX) || token.len() <= TOKEN_PREFIX.len() || !token.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(MfError::SignIn);
    }
    if len == 0 || file_base64.is_empty() {
        return Err(MfError::Empty);
    }
    if len > MAX_FILE {
        return Err(MfError::TooLarge);
    }
    let body = json!({ "file_name": file_name, "file": file_base64, "app": app });
    Ok(HttpRequest {
        url: format!("{SITE}{DESIGNS}"),
        headers: vec![("content-type", "application/json".to_string()), ("authorization", format!("Bearer {token}"))],
        body: body.to_string(),
    })
}

/// MakerFoundry's answer to [`publish`] with HTTP status `status` and body `body`: the draft, or
/// why not (its own sentence for a refusal).
pub fn published(status: u16, body: &str) -> Result<Published, MfError> {
    if !(200..300).contains(&status) {
        return Err(refusal(status, body));
    }
    let v: Value = serde_json::from_str(body).map_err(|_| MfError::NoAnswer)?;
    let id = v["id"].as_u64().ok_or(MfError::NoAnswer)?;
    let url = v["url"].as_str().ok_or(MfError::NoAnswer)?;
    if !on_site(url) {
        return Err(MfError::OffSite);
    }
    Ok(Published { id, url: url.to_string() })
}

/// The error a refusal with HTTP status `status` and body `body` is: MakerFoundry's own sentence
/// (`{"error": {"message"}}`), tidied (one line, at most [`MAX_SENTENCE`] characters), else what
/// its status means.
pub fn refusal(status: u16, body: &str) -> MfError {
    let said = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v.pointer("/error/message").and_then(Value::as_str).map(sentence))
        .filter(|s| !s.is_empty());
    match said {
        Some(s) => MfError::Said(status, s),
        None if status == 401 => MfError::SignIn,
        None if status == 413 => MfError::TooLarge,
        None => MfError::Said(status, format!("HTTP {status}")),
    }
}

/// `s` as one line of at most [`MAX_SENTENCE`] characters, without control characters.
fn sentence(s: &str) -> String {
    let one: String = s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    let one = one.split_whitespace().collect::<Vec<_>>().join(" ");
    if one.chars().count() > MAX_SENTENCE { one.chars().take(MAX_SENTENCE - 1).chain(['…']).collect() } else { one }
}

/// Whether `url` is a page on MakerFoundry's site, the one the request went to: `https://`, the
/// host exactly [`HOST`] (no user name, no port, no other host ending in it), then a path; printable
/// ASCII without spaces or backslashes, at most [`MAX_ADDRESS`] characters.
pub fn on_site(url: &str) -> bool {
    if url.len() > MAX_ADDRESS || !url.bytes().all(|b| b.is_ascii_graphic() && b != b'\\') {
        return false;
    }
    let Some(rest) = url.strip_prefix("https://") else { return false };
    let host_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    &rest[..host_end] == HOST && rest[host_end..].starts_with('/')
}

// ---- PF-12: opening a design file from a MakerFoundry link ----

/// The hosts a design file is opened from.
pub const FILE_HOSTS: [&str; 2] = ["makerfoundry.app", "www.makerfoundry.app"];
/// Where design files live on them.
pub const FILE_PATHS: &str = "/api/v1/";
/// The most a downloaded design may weigh.
pub const MAX_DOWNLOAD: usize = 32 * 1024 * 1024;
/// The length of a file address's access token (`?t=`, lowercase hex).
const TOKEN_HEX: usize = 48;
/// The longest file name taken from a download.
const MAX_NAME: usize = 120;

/// Why a MakerFoundry link wasn't opened. The messages are for the user; the UI shows them
/// translated, and MakerFoundry's own sentences as they are.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum OpenError {
    #[error("That link isn't a design file on MakerFoundry, so it wasn't opened.")]
    NotMakerFoundry,
    #[error("The design is too large to open: at most 32 MB.")]
    TooLarge,
    #[error("PatternFoundry opens SVG and DXF designs from MakerFoundry; that file is neither.")]
    Kind,
    /// MakerFoundry's own sentence (with the HTTP status it came with).
    #[error("MakerFoundry: {1}")]
    Said(u16, String),
    #[error("The design couldn't be downloaded from MakerFoundry: {0}")]
    Network(String),
}

/// The design file address `link` names, rebuilt from its checked parts, or why it isn't one:
/// `https://`, the host exactly one of [`FILE_HOSTS`] (no user name, no port), a path under
/// [`FILE_PATHS`] of `a-z 0-9 - . /` only (no empty, `.` or `..` segment), and at most the access
/// token `?t=` and 48 lowercase hex digits; printable ASCII, at most 2048 characters, no fragment.
pub fn open_address(link: &str) -> Result<String, OpenError> {
    let bad = OpenError::NotMakerFoundry;
    if link.is_empty() || link.len() > MAX_ADDRESS || !link.bytes().all(|b| b.is_ascii_graphic() && b != b'\\') {
        return Err(bad);
    }
    let rest = link.strip_prefix("https://").ok_or(bad.clone())?;
    let host_end = rest.find('/').ok_or(bad.clone())?;
    let host = &rest[..host_end];
    let host = FILE_HOSTS.iter().find(|h| **h == host).ok_or(bad.clone())?;
    let (path, query) = match rest[host_end..].split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (&rest[host_end..], None),
    };
    let tail = path.strip_prefix(FILE_PATHS).ok_or(bad.clone())?;
    let path_ok = path.bytes().all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'/'))
        && !tail.is_empty()
        && tail.split('/').all(|seg| !seg.is_empty() && seg != "." && seg != "..");
    if !path_ok {
        return Err(bad);
    }
    let token = match query {
        None => None,
        Some(q) => {
            let t = q.strip_prefix("t=").ok_or(bad.clone())?;
            if t.len() != TOKEN_HEX || !t.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
                return Err(bad);
            }
            Some(t)
        }
    };
    Ok(match token {
        Some(t) => format!("https://{host}{path}?t={t}"),
        None => format!("https://{host}{path}"),
    })
}

/// The value of `open` in the page address's query `search` (`?open=…&lang=de`), percent-decoded;
/// `None` without one, or when it doesn't decode to text.
pub fn open_param(search: &str) -> Option<String> {
    let q = search.strip_prefix('?').unwrap_or(search);
    let raw = q.split('&').find_map(|kv| kv.strip_prefix("open="))?;
    percent_decode(raw)
}

/// The query `search` without its `open` parameter (`""` when nothing else is left), for the page
/// address once the link has been read.
pub fn without_open(search: &str) -> String {
    let q = search.strip_prefix('?').unwrap_or(search);
    let kept: Vec<&str> = q.split('&').filter(|kv| !kv.is_empty() && *kv != "open" && !kv.starts_with("open=")).collect();
    if kept.is_empty() { String::new() } else { format!("?{}", kept.join("&")) }
}

/// `s` with `%XX` escapes (and `+` for a space, as forms write it) decoded; `None` for a broken
/// escape or bytes that aren't UTF-8.
fn percent_decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' => {
                let hex = s.get(i + 1..i + 3)?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

/// The name a downloaded design opens as: the file name its `Content-Disposition` header gives
/// (`attachment; filename="Tote.svg"`), else the end of its address, without any folder; and only
/// an SVG or DXF ([`OpenError::Kind`] otherwise).
pub fn download_name(content_disposition: Option<&str>, address: &str) -> Result<String, OpenError> {
    let from_header = content_disposition.and_then(disposition_name);
    let from_address = || {
        let path = address.split(['?', '#']).next().unwrap_or_default();
        path.rsplit('/').next().map(str::to_string)
    };
    let name = from_header.or_else(from_address).unwrap_or_default();
    let name = name.rsplit(['/', '\\']).next().unwrap_or_default();
    let name: String = name.chars().filter(|c| !c.is_control()).take(MAX_NAME).collect();
    let name = name.trim().trim_start_matches('.').to_string();
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some("svg" | "dxf") if name.len() > 4 => Ok(name),
        _ => Err(OpenError::Kind),
    }
}

/// The `filename` of a `Content-Disposition` header, quoted or not.
fn disposition_name(h: &str) -> Option<String> {
    let at = h.find("filename=")? + "filename=".len();
    let v = h[at..].trim_start();
    let name = match v.strip_prefix('"') {
        Some(q) => q.split('"').next()?,
        None => v.split(';').next()?.trim(),
    };
    (!name.is_empty()).then(|| name.to_string())
}

/// The error a download refused with HTTP status `status` and body `body` is: MakerFoundry's own
/// sentence (`{"error": {"message"}}`, tidied), else what its status means.
pub fn download_refusal(status: u16, body: &str) -> OpenError {
    let said = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v.pointer("/error/message").and_then(Value::as_str).map(sentence))
        .filter(|s| !s.is_empty());
    match said {
        Some(s) => OpenError::Said(status, s),
        None if status == 413 => OpenError::TooLarge,
        None => OpenError::Said(status, format!("HTTP {status}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_design_is_sent_with_the_session_and_nothing_else() {
        let r = publish(" sfs_abc ", "Skirt.svg", "PHN2Zy8+", 6, "PatternFoundry 0.13.1").unwrap();
        assert_eq!(r.url, "https://makerfoundry.app/api/v1/designs");
        assert!(r.headers.contains(&("authorization", "Bearer sfs_abc".to_string())));
        assert_eq!(r.headers.iter().filter(|h| h.0 == "authorization").count(), 1);
        let body: Value = serde_json::from_str(&r.body).unwrap();
        assert_eq!(body, json!({"file_name": "Skirt.svg", "file": "PHN2Zy8+", "app": "PatternFoundry 0.13.1"}));
        // An Anthropic key, an empty token, or one with a line break in it never leaves.
        for bad in ["sk-ant-api03-abc", "", "sfs_", "sfs_a\r\nx-api-key: 1", "Bearer sfs_x"] {
            assert_eq!(publish(bad, "a.svg", "PA==", 1, "PatternFoundry"), Err(MfError::SignIn), "{bad:?}");
        }
        assert_eq!(publish("sfs_a", "a.svg", "", 0, "PatternFoundry"), Err(MfError::Empty));
        assert_eq!(publish("sfs_a", "a.svg", "PA==", MAX_FILE + 1, "PatternFoundry"), Err(MfError::TooLarge));
        assert!(publish("sfs_a", "a.svg", "PA==", MAX_FILE, "PatternFoundry").is_ok());
    }

    #[test]
    fn file_names_make_meaningful_titles() {
        assert_eq!(file_name("Simple tote bag.patternfoundry", "svg"), "Simple_tote_bag.svg");
        assert_eq!(file_name("/home/me/A-line skirt (v2).svg", ".svg"), "A-line_skirt_v2.svg");
        assert_eq!(file_name("C:\\pats\\Röck.pdf", "svg"), "Röck.svg");
        assert_eq!(file_name("Untitled-1", "svg"), "Untitled-1.svg");
        for nothing in ["", "...", "/", "___", "..svg", "\u{202e}\u{0}"] {
            assert_eq!(file_name(nothing, "svg"), "PatternFoundry_design.svg", "{nothing:?}");
        }
        let long = file_name(&"x".repeat(500), "svg");
        assert_eq!(long, format!("{}.svg", "x".repeat(60)));
        // Nothing that could leave the name: no separators, dots or controls in the stem.
        let n = file_name("../../etc/passwd\n.svg", "svg");
        assert_eq!(n, "passwd.svg");
    }

    #[test]
    fn the_draft_opens_only_on_makerfoundrys_own_site() {
        let ok = published(201, r#"{"id": 7, "url": "https://makerfoundry.app/designs/7/edit", "kind": "laser", "format": "svg"}"#);
        assert_eq!(ok, Ok(Published { id: 7, url: "https://makerfoundry.app/designs/7/edit".into() }));
        for url in [
            "https://makerfoundry.app.evil.com/designs/7",
            "https://evil.com/makerfoundry.app/",
            "https://makerfoundry.app@evil.com/designs/7",
            "https://user@makerfoundry.app/designs/7",
            "https://makerfoundry.app:8443/designs/7",
            "https://makerfoundry.app./designs/7",
            "https://www.makerfoundry.app/designs/7",
            "https://MAKERFOUNDRY.APP/designs/7",
            "http://makerfoundry.app/designs/7",
            "javascript:alert(1)//https://makerfoundry.app/",
            "//makerfoundry.app/designs/7",
            "https://makerfoundry.app",
            "https://makerfoundry.app\\@evil.com/",
            "https://makerfoundry.app/designs/7 x",
            "https://makerfoundry.app/designs/7\n",
            "https://mаkerfoundry.app/designs/7",
        ] {
            let body = json!({"id": 7, "url": url}).to_string();
            assert_eq!(published(201, &body), Err(MfError::OffSite), "{url}");
        }
        let long = format!("https://makerfoundry.app/{}", "a".repeat(3000));
        assert_eq!(published(201, &json!({"id": 1, "url": long}).to_string()), Err(MfError::OffSite));
        for bad in
            ["", "not json", r#"{"url": "https://makerfoundry.app/d/1"}"#, r#"{"id": "7", "url": "https://makerfoundry.app/d"}"#, r#"{"id": 7}"#]
        {
            assert_eq!(published(201, bad), Err(MfError::NoAnswer), "{bad}");
        }
    }

    #[test]
    fn refusals_show_makerfoundrys_own_sentence() {
        let said = |status, msg: &str| published(status, &json!({"error": {"message": msg}}).to_string());
        assert_eq!(said(401, "Sign in again."), Err(MfError::Said(401, "Sign in again.".into())));
        assert_eq!(
            said(422, "That isn't a design MakerFoundry can read."),
            Err(MfError::Said(422, "That isn't a design MakerFoundry can read.".into()))
        );
        // One line, no controls, not endless.
        assert_eq!(said(429, "Too many\r\n\u{1b}[31m at once"), Err(MfError::Said(429, "Too many [31m at once".into())));
        let MfError::Said(_, long) = said(503, &"z".repeat(5000)).unwrap_err() else { panic!() };
        assert_eq!(long.chars().count(), 300);
        // Without a sentence: what the status means.
        assert_eq!(published(401, "<html>"), Err(MfError::SignIn));
        assert_eq!(published(413, ""), Err(MfError::TooLarge));
        assert_eq!(published(503, "{}"), Err(MfError::Said(503, "HTTP 503".into())));
        assert_eq!(MfError::Said(400, "Bad.".into()).to_string(), "MakerFoundry: Bad.");
    }

    #[test]
    fn only_makerfoundry_design_files_open_rebuilt_from_their_parts() {
        let t = "0123456789abcdef0123456789abcdef0123456789abcdef";
        for ok in [
            "https://makerfoundry.app/api/v1/designs/1/file/test.svg".to_string(),
            "https://www.makerfoundry.app/api/v1/designs/12/file/tote-bag.svg".to_string(),
            format!("https://makerfoundry.app/api/v1/designs/12/file/a.dxf?t={t}"),
        ] {
            assert_eq!(open_address(&ok).as_deref(), Ok(ok.as_str()));
        }
        let long = format!("https://makerfoundry.app/api/v1/{}", "a".repeat(2048));
        for bad in [
            "",
            "http://makerfoundry.app/api/v1/designs/1/file/a.svg",
            "HTTPS://makerfoundry.app/api/v1/designs/1/file/a.svg",
            "https://MakerFoundry.app/api/v1/designs/1/file/a.svg",
            "https://makerfoundry.app.evil.com/api/v1/designs/1/file/a.svg",
            "https://evil.com/api/v1/designs/1/file/a.svg?x=makerfoundry.app",
            "https://user@makerfoundry.app/api/v1/designs/1/file/a.svg",
            "https://makerfoundry.app@evil.com/api/v1/designs/1/file/a.svg",
            "https://makerfoundry.app:443/api/v1/designs/1/file/a.svg",
            "https://makerfoundry.app./api/v1/designs/1/file/a.svg",
            "https://makerfoundry\u{430}.app/api/v1/designs/1/file/a.svg",
            "https://makerfoundry.app",
            "https://makerfoundry.app/",
            "https://makerfoundry.app/api/v1/",
            "https://makerfoundry.app/designs/1",
            "https://makerfoundry.app/api/v2/designs/1/file/a.svg",
            "https://makerfoundry.app/api/v1/../../admin",
            "https://makerfoundry.app/api/v1/./designs",
            "https://makerfoundry.app/api/v1/designs//1",
            "https://makerfoundry.app/api/v1/designs/1/file/A.svg",
            "https://makerfoundry.app/api/v1/designs/1/file/a%2e.svg",
            "https://makerfoundry.app/api/v1/designs/1/file/a b.svg",
            "https://makerfoundry.app/api/v1/designs/1/file/a.svg\n",
            "https://makerfoundry.app/api/v1/designs/1/file/a\\b.svg",
            "https://makerfoundry.app/api/v1/designs/1/file/a.svg#x",
            "https://makerfoundry.app/api/v1/designs/1/file/a.svg?t=abc",
            "https://makerfoundry.app/api/v1/designs/1/file/a.svg?t=0123456789ABCDEF0123456789abcdef0123456789abcdef",
            "https://makerfoundry.app/api/v1/designs/1/file/a.svg?t=0123456789abcdef0123456789abcdef0123456789abcdef&x=1",
            "https://makerfoundry.app/api/v1/designs/1/file/a.svg?x=1",
            "https://makerfoundry.app?/api/v1/designs/1/file/a.svg",
            "javascript:alert(1)//makerfoundry.app/api/v1/x",
            "//makerfoundry.app/api/v1/designs/1/file/a.svg",
            long.as_str(),
        ] {
            assert_eq!(open_address(bad), Err(OpenError::NotMakerFoundry), "{bad:?}");
        }
    }

    #[test]
    fn the_link_is_read_from_the_page_address_and_taken_out() {
        let enc = "?open=https%3A%2F%2Fmakerfoundry.app%2Fapi%2Fv1%2Fdesigns%2F1%2Ffile%2Ftest.svg&lang=de";
        assert_eq!(open_param(enc).as_deref(), Some("https://makerfoundry.app/api/v1/designs/1/file/test.svg"));
        assert_eq!(without_open(enc), "?lang=de");
        assert_eq!(without_open("?open=x"), "");
        assert_eq!(without_open("?opener=1&open=x"), "?opener=1");
        assert_eq!(open_param("?lang=de"), None);
        assert_eq!(open_param("?open=%zz"), None);
        assert_eq!(open_param("?open=%ff"), None);
        assert_eq!(open_param("?open=%4"), None);
        // `+` decodes to a space, which the address check then refuses.
        assert_eq!(open_param("?open=a+b").as_deref(), Some("a b"));
    }

    #[test]
    fn a_download_is_named_from_its_header_or_address_and_must_be_svg_or_dxf() {
        let a = "https://makerfoundry.app/api/v1/designs/1/file/test.svg?t=00";
        assert_eq!(download_name(Some("attachment; filename=\"Tote bag.svg\""), a).as_deref(), Ok("Tote bag.svg"));
        assert_eq!(download_name(Some("attachment; filename=Tote.DXF"), a).as_deref(), Ok("Tote.DXF"));
        assert_eq!(download_name(None, a).as_deref(), Ok("test.svg"));
        assert_eq!(download_name(Some("attachment"), a).as_deref(), Ok("test.svg"));
        assert_eq!(download_name(Some("attachment; filename=\"../../etc/x.svg\""), a).as_deref(), Ok("x.svg"));
        assert_eq!(download_name(Some("attachment; filename=\"C:\\\\x\\\\y.svg\""), a).as_deref(), Ok("y.svg"));
        assert_eq!(download_name(Some("attachment; filename=\"a\u{7}b.svg\""), a).as_deref(), Ok("ab.svg"));
        for bad in ["attachment; filename=\"x.exe\"", "attachment; filename=\".svg\"", "attachment; filename=\"svg\""] {
            assert_eq!(download_name(Some(bad), a), Err(OpenError::Kind), "{bad}");
        }
        assert_eq!(download_name(None, "https://makerfoundry.app/api/v1/designs/1/file/x.pdf"), Err(OpenError::Kind));
    }

    #[test]
    fn a_refused_download_says_makerfoundrys_sentence() {
        let body = r#"{"error":{"message":"That design isn't on MakerFoundry."}}"#;
        for status in [403, 404, 429, 502, 503] {
            assert_eq!(download_refusal(status, body).to_string(), "MakerFoundry: That design isn't on MakerFoundry.");
        }
        assert_eq!(download_refusal(404, "<html>").to_string(), "MakerFoundry: HTTP 404");
        assert_eq!(download_refusal(413, ""), OpenError::TooLarge);
        // One line, without control characters, at most 300 characters.
        let messy = serde_json::json!({"error": {"message": format!("Gone.\n\u{1b}[31m{}", "x".repeat(400))}}).to_string();
        let said = download_refusal(404, &messy).to_string();
        assert!(said.starts_with("MakerFoundry: Gone. [31mxxx") && !said.chars().any(char::is_control), "{said}");
        assert_eq!(said.chars().count(), "MakerFoundry: ".len() + 300);
    }
}
