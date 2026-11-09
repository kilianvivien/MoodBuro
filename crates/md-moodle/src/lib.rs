//! Moodle protocol and navigation policy. No dependency on Tauri.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use url::Url;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Enter an HTTPS Moodle base URL, without a query or fragment.")]
    InvalidUrl,
    #[error("The sign-in response is invalid or belongs to another sign-in attempt.")]
    InvalidCallback,
    #[error("The Moodle site could not be reached or returned an unexpected response.")]
    Network,
    #[error(
        "This site could not authorise mobile web services. Ask its administrator to enable them."
    )]
    WebServices,
}

pub fn normalize_site(input: &str) -> Result<Url, Error> {
    let input = input.trim();
    let candidate = if input.contains("://") {
        input.to_owned()
    } else {
        format!("https://{input}")
    };
    let mut url = Url::parse(&candidate).map_err(|_| Error::InvalidUrl)?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(Error::InvalidUrl);
    }
    let path = url.path().trim_end_matches('/').to_owned();
    // Retain subdirectory installations; the API's wwwroot is authoritative.
    url.set_path(&format!("{path}/"));
    Ok(url)
}

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct Tokens {
    pub token: String,
    pub private: Option<String>,
}

pub fn parse_launch_token(callback: &Url, wwwroot: &str, passport: &str) -> Result<Tokens, Error> {
    if !matches!(callback.scheme(), "moodlemobile" | "moodledesk") {
        return Err(Error::InvalidCallback);
    }
    let encoded = callback
        .as_str()
        .split_once("://token=")
        .ok_or(Error::InvalidCallback)?
        .1;
    let bytes = zeroize::Zeroizing::new(
        STANDARD
            .decode(encoded)
            .map_err(|_| Error::InvalidCallback)?,
    );
    let raw = std::str::from_utf8(&bytes).map_err(|_| Error::InvalidCallback)?;
    let parts: Vec<_> = raw.split(":::").collect();
    let expected = format!(
        "{:x}",
        md5::compute(format!("{}{passport}", wwwroot.trim_end_matches('/')))
    );
    if !(2..=3).contains(&parts.len())
        || parts[0] != expected
        || parts[1].is_empty()
        || parts[1].len() > 256
        || parts.get(2).is_some_and(|v| v.is_empty() || v.len() > 256)
    {
        return Err(Error::InvalidCallback);
    }
    Ok(Tokens {
        token: parts[1].into(),
        private: parts.get(2).map(|v| (*v).into()),
    })
}

#[derive(Debug, PartialEq, Eq)]
pub enum Navigation {
    Allow,
    External,
    Block,
}
pub fn navigation(site: &Url, target: &Url) -> Navigation {
    if target.scheme() != "https" || !target.username().is_empty() || target.password().is_some() {
        return Navigation::Block;
    }
    if site.origin() == target.origin() {
        Navigation::Allow
    } else {
        Navigation::External
    }
}

#[derive(Deserialize)]
pub struct PublicConfig {
    pub wwwroot: String,
    pub sitename: String,
    pub launchurl: Option<String>,
}
#[derive(Deserialize)]
pub struct SiteInfo {
    pub userid: i64,
    pub fullname: String,
    pub sitename: String,
}

pub struct Client {
    base: Url,
    http: reqwest::Client,
}
impl Client {
    pub fn new(base: Url) -> Result<Self, Error> {
        let http = reqwest::Client::builder()
            .https_only(true)
            .timeout(std::time::Duration::from_secs(15))
            .user_agent(concat!("MoodBuro/", env!("CARGO_PKG_VERSION")))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| Error::Network)?;
        Ok(Self { base, http })
    }
    pub async fn public_config(&self) -> Result<PublicConfig, Error> {
        let endpoint = self
            .base
            .join("lib/ajax/service.php")
            .map_err(|_| Error::InvalidUrl)?;
        let response: serde_json::Value = self.http.post(endpoint)
            .json(&serde_json::json!([{"index":0,"methodname":"tool_mobile_get_public_config","args":{}}]))
            .send().await.map_err(|_| Error::Network)?.error_for_status().map_err(|_| Error::Network)?
            .json().await.map_err(|_| Error::Network)?;
        if response[0]["error"].as_bool() == Some(true) {
            return Err(Error::WebServices);
        }
        serde_json::from_value(response[0]["data"].clone()).map_err(|_| Error::WebServices)
    }
    pub async fn site_info(&self, token: &str) -> Result<SiteInfo, Error> {
        let endpoint = self
            .base
            .join("webservice/rest/server.php")
            .map_err(|_| Error::InvalidUrl)?;
        self.http
            .post(endpoint)
            .form(&[
                ("wstoken", token),
                ("wsfunction", "core_webservice_get_site_info"),
                ("moodlewsrestformat", "json"),
            ])
            .send()
            .await
            .map_err(|_| Error::Network)?
            .error_for_status()
            .map_err(|_| Error::Network)?
            .json()
            .await
            .map_err(|_| Error::WebServices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalizes_subdirectory_sites() {
        assert_eq!(
            normalize_site(" example.org/campus ").unwrap().as_str(),
            "https://example.org/campus/"
        );
        assert_eq!(
            normalize_site("https://moodle.insp.gouv.fr/")
                .unwrap()
                .as_str(),
            "https://moodle.insp.gouv.fr/"
        );
    }
    #[test]
    fn rejects_unsafe_urls() {
        for input in [
            "http://example.org",
            "https://user:pass@example.org",
            "https://example.org/?token=x",
            "file:///etc/passwd",
            "https://example.org/#secret",
        ] {
            assert!(normalize_site(input).is_err(), "{input}");
        }
    }
    #[test]
    fn guards_navigation_by_origin() {
        let site = normalize_site("example.org/campus").unwrap();
        assert_eq!(
            navigation(
                &site,
                &Url::parse("https://example.org/course/view.php?id=2").unwrap()
            ),
            Navigation::Allow
        );
        for target in [
            "https://example.org.evil.test/",
            "https://example.org:444/",
            "https://idp.example.org/",
        ] {
            assert_eq!(
                navigation(&site, &Url::parse(target).unwrap()),
                Navigation::External
            );
        }
        for target in [
            "javascript:alert(1)",
            "file:///tmp/a",
            "http://example.org/",
        ] {
            assert_eq!(
                navigation(&site, &Url::parse(target).unwrap()),
                Navigation::Block
            );
        }
    }
    #[test]
    fn verifies_callback_and_rejects_tampering() {
        let signature = format!("{:x}", md5::compute("https://example.orgnonce"));
        let callback = Url::parse(&format!(
            "moodlemobile://token={}",
            STANDARD.encode(format!("{signature}:::abc:::private"))
        ))
        .unwrap();
        let tokens = parse_launch_token(&callback, "https://example.org/", "nonce").unwrap();
        assert_eq!(tokens.token, "abc");
        assert!(parse_launch_token(&callback, "https://example.org", "other").is_err());
        for raw in [
            signature.as_str(),
            "wrong:::abc",
            "wrong:::abc:::private:::extra",
        ] {
            let callback =
                Url::parse(&format!("moodlemobile://token={}", STANDARD.encode(raw))).unwrap();
            assert!(parse_launch_token(&callback, "https://example.org", "nonce").is_err());
        }
    }
}
