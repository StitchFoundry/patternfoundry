//! The page's content security policy, checked against `index.html` (PF-12): the page has none
//! today, but if one is ever added, `connect-src` must let the app reach MakerFoundry's two hosts
//! (opening `?open=` links) and every inline `<script>` must be allowed by its SHA-256 in
//! `script-src`, or the browser silently refuses them. (Trunk's build adds its own loader script;
//! a policy shipped with the built page must allow that one too.)
// Test code: unwrapping and panicking on failure is fine here (AGENTS.md › Robustness).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use sha2::{Digest, Sha256};

/// The hosts the page must be able to fetch from.
const MUST_CONNECT: [&str; 4] =
    ["https://makerfoundry.app", "https://www.makerfoundry.app", pf_ai::makerfoundry::STAGING_SITE, pf_ai::makerfoundry::STAGING_CLOUD];

/// What is wrong with the policy of the page `html`, if it has one.
fn problems(html: &str) -> Vec<String> {
    let lower = html.to_ascii_lowercase();
    let Some(at) = lower.find("http-equiv=\"content-security-policy\"") else { return Vec::new() };
    let tag_start = lower[..at].rfind('<').unwrap();
    let tag_end = tag_start + lower[tag_start..].find('>').unwrap();
    let tag = &html[tag_start..tag_end];
    let content = tag.split("content=\"").nth(1).and_then(|c| c.split('"').next()).unwrap_or_default();
    let directive = |name: &str| -> Vec<String> {
        content
            .split(';')
            .map(str::trim)
            .find_map(|d| d.strip_prefix(name).filter(|r| r.is_empty() || r.starts_with(' ')))
            .map(|r| r.split_whitespace().map(str::to_string).collect())
            .unwrap_or_default()
    };
    let mut out = Vec::new();
    let connect = directive("connect-src");
    let default = directive("default-src");
    let connect = if connect.is_empty() { default.clone() } else { connect };
    for host in MUST_CONNECT {
        if !connect.iter().any(|s| s == host || s == "https:" || s == "*") {
            out.push(format!("connect-src doesn't allow {host}"));
        }
    }
    let scripts = directive("script-src");
    let scripts = if scripts.is_empty() { default } else { scripts };
    let mut rest = html;
    while let Some(open) = rest.find("<script") {
        let after = &rest[open..];
        let head_end = after.find('>').unwrap();
        let head = &after[..head_end];
        let body_end = after.find("</script>").unwrap();
        if !head.contains(" src=") {
            let body = &after[head_end + 1..body_end];
            let hash = format!("'sha256-{}'", vectorcraft_format::base64_encode(&Sha256::digest(body.as_bytes())));
            if !scripts.contains(&hash) {
                out.push(format!("script-src doesn't allow the inline script {hash}"));
            }
        }
        rest = &after[body_end + "</script>".len()..];
    }
    out
}

#[test]
fn the_pages_policy_lets_it_reach_makerfoundry_and_run_its_scripts() {
    let page = include_str!("../index.html");
    assert_eq!(problems(page), Vec::<String>::new());
}

#[test]
fn a_policy_missing_a_host_or_a_script_hash_is_caught() {
    let script = "console.log(1)";
    let hash = format!("'sha256-{}'", vectorcraft_format::base64_encode(&Sha256::digest(script.as_bytes())));
    let page = |policy: &str| {
        format!(
            "<html><head><meta http-equiv=\"Content-Security-Policy\" content=\"{policy}\"><script>{script}</script><script src=\"a.js\"></script></head></html>"
        )
    };
    let staging = format!("{} {}", pf_ai::makerfoundry::STAGING_SITE, pf_ai::makerfoundry::STAGING_CLOUD);
    let good = format!("default-src 'self'; connect-src 'self' https://makerfoundry.app https://www.makerfoundry.app {staging}; script-src 'self' {hash}");
    assert_eq!(problems(&page(&good)), Vec::<String>::new());
    let no_www = good.replace(" https://www.makerfoundry.app", "");
    assert_eq!(problems(&page(&no_www)), ["connect-src doesn't allow https://www.makerfoundry.app"]);
    let no_hash = good.replace(&format!(" {hash}"), "");
    assert_eq!(problems(&page(&no_hash)).len(), 1);
    // Without connect-src, default-src decides.
    assert_eq!(problems(&page("default-src 'self'")).len(), 5);
    // No policy: nothing to check.
    assert!(problems("<html><script>x</script></html>").is_empty());
}
