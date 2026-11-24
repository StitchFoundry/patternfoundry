//! The pure part of the in-app update check: is a release manifest's version newer than the running
//! one, and does a downloaded file match the hash the manifest names. No network, no clock, no file
//! access; the host fetches the manifest and shows the message. (Checking the manifest's Ed25519
//! signature is the next slice; until then nothing here may be shown to a user as "verified".)

use serde::Deserialize;
use sha2::{Digest, Sha256};

/// A release manifest as the hub serves it (the fields this check reads).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Manifest {
    pub version: String,
    #[serde(default)]
    pub notes: String,
    /// The release page to open; the app never downloads or installs by itself.
    #[serde(default)]
    pub url: String,
    /// Lower-case hex SHA-256 of the download.
    #[serde(default)]
    pub sha256: String,
}

impl Manifest {
    /// Reads a manifest from JSON; `None` for anything unreadable.
    pub fn parse(json: &[u8]) -> Option<Manifest> {
        serde_json::from_slice(json).ok()
    }
}

/// `x.y.z` (an optional leading `v`); anything else, such as `1.2`, `1.2.3-beta` or `1.x.3`, is `None`.
pub fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
    let s = s.trim();
    let s = s.strip_prefix('v').unwrap_or(s);
    let mut parts = s.split('.');
    let mut next = || {
        let p = parts.next()?;
        (!p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())).then(|| p.parse::<u64>().ok())?
    };
    let v = (next()?, next()?, next()?);
    parts.next().is_none().then_some(v)
}

/// Only a strictly newer, readable version counts; a downgrade, the same version and anything
/// unreadable are "not newer".
pub fn is_newer(running: &str, offered: &str) -> bool {
    match (parse_version(running), parse_version(offered)) {
        (Some(r), Some(o)) => o > r,
        _ => false,
    }
}

/// The manifest, if it offers a strictly newer version than `running`.
pub fn newer_release<'a>(running: &str, manifest: &'a Manifest) -> Option<&'a Manifest> {
    is_newer(running, &manifest.version).then_some(manifest)
}

/// Does `data` hash to `expected_hex` (SHA-256, hex in either case)? An unreadable hash never matches.
pub fn sha256_matches(data: &[u8], expected_hex: &str) -> bool {
    let digest = Sha256::digest(data);
    let expected = expected_hex.trim().to_ascii_lowercase();
    if expected.len() != digest.len() * 2 {
        return false;
    }
    let got: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    got == expected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_strictly_newer_version_counts() {
        assert!(is_newer("0.1.3", "0.1.4"));
        assert!(is_newer("0.1.3", "v0.2.0"));
        assert!(is_newer("0.9.9", "0.10.0"), "numbers, not text");
        assert!(!is_newer("0.1.3", "0.1.3"));
    }

    #[test]
    fn a_downgrade_is_not_an_update() {
        assert!(!is_newer("0.2.0", "0.1.9"));
        assert!(!is_newer("1.0.0", "0.99.99"));
    }

    #[test]
    fn a_garbled_version_is_not_newer() {
        for bad in ["", "1.2", "1.2.3.4", "1.2.3-beta", "1.x.3", "-1.0.0", "1..3", "99999999999999999999.0.0"] {
            assert!(!is_newer("0.1.3", bad), "{bad}");
            assert!(!is_newer(bad, "9.9.9"), "{bad}");
        }
    }

    #[test]
    fn manifest_parsing_and_selection() {
        let m = Manifest::parse(br#"{"version":"0.2.0","notes":"Faster tiling","url":"https://example.org/r"}"#);
        let m = m.expect("readable");
        assert_eq!(newer_release("0.1.3", &m).map(|r| r.notes.as_str()), Some("Faster tiling"));
        assert!(newer_release("0.2.0", &m).is_none());
        assert!(Manifest::parse(b"not json").is_none());
        assert!(Manifest::parse(br#"{"notes":"no version"}"#).is_none());
    }

    #[test]
    fn download_hash_is_checked() {
        // SHA-256("abc"), FIPS 180-4 example.
        let abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert!(sha256_matches(b"abc", abc));
        assert!(sha256_matches(b"abc", &abc.to_ascii_uppercase()));
        assert!(!sha256_matches(b"abd", abc));
        assert!(!sha256_matches(b"abc", ""));
        assert!(!sha256_matches(b"abc", "ba78"));
    }
}
