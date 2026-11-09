//! Help → project links: this app's GitHub repository (source code, issues and releases), the
//! MakerFoundry YouTube channel (videos) and the Buy Me a Coffee page (support the project). The
//! commands return the URL; the frontend opens it.

use serde_json::{Value, json};

use super::*;

/// This app's source repository.
pub const GITHUB_URL: &str = "https://github.com/StitchFoundry/patternfoundry";
/// The MakerFoundry YouTube channel: videos for the Foundry apps.
pub const YOUTUBE_URL: &str = "https://www.youtube.com/@MakerFoundryApp";
/// The Buy Me a Coffee page: support the project.
pub const DONATE_URL: &str = "https://buymeacoffee.com/bwattendorf";

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(query "help.github", "PatternFoundry on GitHub", ["Help"], None, "{} → {url} source code, issues and releases", always, |_, _| url(GITHUB_URL.into())),
        cmd!(query "help.youtube", "MakerFoundry on YouTube", ["Help"], None, "{} → {url} the MakerFoundry YouTube channel: videos", always, |_, _| url(YOUTUBE_URL.into())),
        cmd!(query "help.donate", "Support PatternFoundry (Buy Me a Coffee)", ["Help"], None, "{} → {url} the Buy Me a Coffee page: support the project", always, |_, _| url(DONATE_URL.into())),
        cmd!(query "help.links", "Links", [], None, "{} → {github, youtube, donate}", always, |_, _| Ok(json!({ "github": GITHUB_URL, "youtube": YOUTUBE_URL, "donate": DONATE_URL }))),
    ]
}

fn url(u: String) -> Result<Value> {
    Ok(json!({ "url": u }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links() {
        let mut s = Session::new();
        assert_eq!(s.execute("help.github", &json!({})).unwrap()["url"], "https://github.com/StitchFoundry/patternfoundry");
        assert_eq!(s.execute("help.youtube", &json!({})).unwrap()["url"], "https://www.youtube.com/@MakerFoundryApp");
        assert_eq!(s.execute("help.donate", &json!({})).unwrap()["url"], "https://buymeacoffee.com/bwattendorf");
        let links = s.execute("help.links", &json!({})).unwrap();
        assert_eq!(links["github"], "https://github.com/StitchFoundry/patternfoundry");
        assert_eq!(links["youtube"], "https://www.youtube.com/@MakerFoundryApp");
        assert_eq!(links["donate"], "https://buymeacoffee.com/bwattendorf");
        assert!(s.execute("help.discord", &json!({})).is_err(), "no community links of VectorCraft's makers");
    }
}
