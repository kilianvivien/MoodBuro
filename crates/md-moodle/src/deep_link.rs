//! Untrusted OS/CLI links are validated before they reach a webview or the shell.
use crate::{normalize_site, Error};
use url::Url;

#[derive(Clone, Debug)]
pub struct Link {
    target: Url,
}
pub enum Route {
    Site { id: String, target: Url },
    Onboarding { base: Url },
}
impl Link {
    pub fn parse(input: &str) -> Result<Self, Error> {
        if input.len() > 4096 || input.chars().any(|c| c.is_control() || c == '\\') {
            return Err(Error::InvalidDeepLink);
        }
        let (scheme, rest) = input.split_once("://").ok_or(Error::InvalidDeepLink)?;
        if !scheme.eq_ignore_ascii_case("moodledesk") {
            return Err(Error::InvalidDeepLink);
        }
        let raw_path = rest
            .split_once('/')
            .map_or("", |(_, path)| path)
            .split(['?', '#'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if raw_path
            .split('/')
            .any(|segment| matches!(segment, "." | ".."))
            || ["%2f", "%5c", "%2e", "%25", "%00"]
                .iter()
                .any(|s| raw_path.contains(s))
        {
            return Err(Error::InvalidDeepLink);
        }
        let target = Url::parse(&format!("https://{rest}")).map_err(|_| Error::InvalidDeepLink)?;
        if target.host_str().is_none()
            || !target.username().is_empty()
            || target.password().is_some()
        {
            return Err(Error::InvalidDeepLink);
        }
        let path = target.path().to_ascii_lowercase();
        // Do not let a second decoding pass or server-side path normalization escape a subdirectory.
        if ["%2f", "%5c", "%2e", "%25", "%00"]
            .iter()
            .any(|s| path.contains(s))
            || target.query_pairs().any(|(k, _)| {
                matches!(
                    k.to_ascii_lowercase().as_str(),
                    "token" | "wstoken" | "privatetoken" | "key" | "passport" | "sesskey"
                )
            })
        {
            return Err(Error::InvalidDeepLink);
        }
        Ok(Self { target })
    }
    /// Pick the longest matching installation root. Same-host sibling installs
    /// never receive each other's pages. Unknown hosts only prefill HTTPS onboarding.
    pub fn route<'a>(
        &self,
        sites: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Result<Route, Error> {
        let mut matches = Vec::new();
        let mut known_origin = false;
        for (id, root) in sites {
            let base = normalize_site(root)?;
            if base.origin() == self.target.origin() {
                known_origin = true;
                if self.target.path().starts_with(base.path()) {
                    matches.push((base.path().len(), id));
                }
            }
        }
        matches.sort_by_key(|(len, _)| *len);
        if let Some((len, id)) = matches.pop() {
            if matches.last().is_some_and(|(other, _)| *other == len) {
                return Err(Error::InvalidDeepLink);
            }
            return Ok(Route::Site {
                id: id.to_owned(),
                target: self.target.clone(),
            });
        }
        if known_origin {
            return Err(Error::InvalidDeepLink);
        }
        let mut base = self.target.clone();
        base.set_path("/");
        base.set_query(None);
        base.set_fragment(None);
        Ok(Route::Onboarding { base })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routes_to_longest_root_and_preserves_moodle_query() {
        let link =
            Link::parse("moodledesk://EXAMPLE.org/campus/course/view.php?id=4#topic-2").unwrap();
        let route = link
            .route([
                ("root", "https://example.org/"),
                ("campus", "https://example.org/campus/"),
            ])
            .unwrap();
        let Route::Site { id, target } = route else {
            panic!("expected saved site")
        };
        assert_eq!(id, "campus");
        assert_eq!(
            target.as_str(),
            "https://example.org/campus/course/view.php?id=4#topic-2"
        );
    }
    #[test]
    fn unknown_host_only_prefills_origin_and_port() {
        let link = Link::parse("moodledesk://new.example.org:8443/course/view.php?id=4").unwrap();
        let Route::Onboarding { base } = link.route([]).unwrap() else {
            panic!("expected onboarding")
        };
        assert_eq!(base.as_str(), "https://new.example.org:8443/");
    }
    #[test]
    fn rejects_unsafe_links_and_sibling_installs() {
        for input in [
            "https://example.org/",
            "moodledesk://user:password@example.org/",
            "moodledesk://example.org/?TOKEN=secret",
            "moodledesk://example.org/?%6bey=secret",
            "moodledesk://example.org/campus/%252e%252e/outside",
            "moodledesk://example.org/campus/%2Foutside",
            "moodledesk://example.org/\\evil",
            "moodledesk://example.org/\n",
            "moodledesk://example.org/campus/../other/",
        ] {
            assert!(Link::parse(input).is_err(), "{input}");
        }
        assert!(Link::parse(&format!("moodledesk://example.org/{}", "a".repeat(4096))).is_err());
        for path in ["/other/", "/campus-evil/"] {
            assert!(Link::parse(&format!("moodledesk://example.org{path}"))
                .unwrap()
                .route([("campus", "https://example.org/campus/")])
                .is_err());
        }
        assert!(Link::parse("moodledesk://example.org/")
            .unwrap()
            .route([
                ("one", "https://example.org/"),
                ("two", "https://example.org/")
            ])
            .is_err());
    }
}

#[cfg(test)]
mod restoration_tests {
    use super::*;
    #[test]
    fn autologin_keeps_requested_page_and_refuses_outside_roots() {
        let base = normalize_site("example.org/campus").unwrap();
        let key = crate::AutologinKey {
            key: "test-key".into(),
            autologinurl: "https://example.org/campus/admin/tool/mobile/autologin.php".into(),
        };
        let destination = base.join("course/view.php?id=4").unwrap();
        let target = crate::autologin_target_to(&base, &key, 2, &destination).unwrap();
        assert!(target
            .query_pairs()
            .any(|(k, v)| k == "urltogo" && v == destination.as_str()));
        for destination in ["https://evil.example.org/", "https://example.org/other/"] {
            assert!(
                crate::autologin_target_to(&base, &key, 2, &Url::parse(destination).unwrap())
                    .is_err()
            );
        }
    }
}
