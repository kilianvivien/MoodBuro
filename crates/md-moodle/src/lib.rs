//! Moodle protocol and navigation policy. No dependency on Tauri.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{de::DeserializeOwned, Deserialize};
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
    #[error("Your Moodle session has expired. Reconnect this organisation in Settings.")]
    InvalidToken,
    #[error("Moodle is limiting requests. Please try again shortly.")]
    RateLimited,
    #[error("This account does not have permission for that Moodle service.")]
    AccessDenied,
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
    // Stock callbacks are a few hundred bytes. Bound input before allocating.
    if encoded.len() > 2048 {
        return Err(Error::InvalidCallback);
    }
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

#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct AutologinKey {
    pub key: String,
    pub autologinurl: String,
}

/// Never expose a one-use login URL to the shell or an unrelated origin.
pub fn autologin_target(base: &Url, login: &AutologinKey, user_id: i64) -> Result<Url, Error> {
    let mut target = Url::parse(&login.autologinurl).map_err(|_| Error::InvalidUrl)?;
    let expected = base
        .join("admin/tool/mobile/autologin.php")
        .map_err(|_| Error::InvalidUrl)?;
    if target.origin() != base.origin()
        || target.path() != expected.path()
        || !target.username().is_empty()
        || target.password().is_some()
        || target.query().is_some()
        || target.fragment().is_some()
        || login.key.is_empty()
    {
        return Err(Error::InvalidUrl);
    }
    target
        .query_pairs_mut()
        .append_pair("userid", &user_id.to_string())
        .append_pair("key", &login.key)
        .append_pair(
            "urltogo",
            base.join("my/").map_err(|_| Error::InvalidUrl)?.as_str(),
        );
    Ok(target)
}

pub struct Client {
    base: Url,
    http: reqwest::Client,
    permits: tokio::sync::Semaphore,
}
impl Client {
    pub fn new(base: Url) -> Result<Self, Error> {
        let origin = base.origin();
        let http = reqwest::Client::builder()
            .https_only(true)
            .timeout(std::time::Duration::from_secs(15))
            // Moodle's stock autologin endpoint requires the compatibility marker.
            // MoodBuro remains explicitly identified (PRD SEC-11).
            .user_agent(concat!(
                "MoodBuro/",
                env!("CARGO_PKG_VERSION"),
                " MoodleMobile"
            ))
            .redirect(reqwest::redirect::Policy::custom(move |attempt| {
                if attempt.previous().len() >= 5
                    || attempt.url().scheme() != "https"
                    || attempt.url().origin() != origin
                {
                    attempt.stop()
                } else {
                    attempt.follow()
                }
            }))
            .build()
            .map_err(|_| Error::Network)?;
        Ok(Self {
            base,
            http,
            permits: tokio::sync::Semaphore::new(4),
        })
    }
    pub async fn public_config(&self) -> Result<PublicConfig, Error> {
        let endpoint = self
            .base
            .join("lib/ajax/service.php")
            .map_err(|_| Error::InvalidUrl)?;
        let response: serde_json::Value = self.http.post(endpoint)
            .json(&serde_json::json!([{"index":0,"methodname":"tool_mobile_get_public_config","args":{}}]))
            .send().await.map_err(|_| Error::Network)?.error_for_status().map_err(|_| Error::Network)?
            .json().await.map_err(|_| Error::WebServices)?;
        if response[0]["error"].as_bool() == Some(true) {
            return Err(Error::WebServices);
        }
        serde_json::from_value(response[0]["data"].clone()).map_err(|_| Error::WebServices)
    }
    /// For Moodle 5.1+ installations whose public document root is exposed explicitly.
    pub async fn discover(base: Url) -> Result<(Url, PublicConfig), Error> {
        let client = Self::new(base.clone())?;
        match client.public_config().await {
            Ok(config) => Ok((base, config)),
            Err(original) => {
                if base.path().trim_end_matches('/').ends_with("/public") {
                    return Err(original);
                }
                let public = base.join("public/").map_err(|_| Error::InvalidUrl)?;
                let config = Self::new(public.clone())?
                    .public_config()
                    .await
                    .map_err(|_| original)?;
                Ok((public, config))
            }
        }
    }
    async fn call<T: DeserializeOwned>(
        &self,
        token: &str,
        function: &str,
        args: &[(&str, &str)],
        retry: bool,
    ) -> Result<T, Error> {
        let _permit = self.permits.acquire().await.map_err(|_| Error::Network)?;
        let endpoint = self
            .base
            .join("webservice/rest/server.php")
            .map_err(|_| Error::InvalidUrl)?;
        let mut form = vec![
            ("wstoken", token),
            ("wsfunction", function),
            ("moodlewsrestformat", "json"),
        ];
        form.extend_from_slice(args);
        let attempts = if retry { 3 } else { 1 };
        for attempt in 0..attempts {
            let result = self.http.post(endpoint.clone()).form(&form).send().await;
            match result {
                Ok(response) => {
                    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        return Err(Error::RateLimited);
                    }
                    if response.status().is_server_error() && attempt + 1 < attempts {
                        backoff(attempt).await;
                        continue;
                    }
                    let response = response.error_for_status().map_err(|_| Error::Network)?;
                    let value: serde_json::Value =
                        response.json().await.map_err(|_| Error::WebServices)?;
                    return decode_response(value);
                }
                Err(_) if attempt + 1 < attempts => backoff(attempt).await,
                Err(_) => return Err(Error::Network),
            }
        }
        Err(Error::Network)
    }
    pub async fn site_info(&self, token: &str) -> Result<SiteInfo, Error> {
        self.call(token, "core_webservice_get_site_info", &[], true)
            .await
    }
    pub async fn autologin_key(&self, token: &str, private: &str) -> Result<AutologinKey, Error> {
        // Minting keys is rate-limited server-side; never automatically retry it.
        self.call(
            token,
            "tool_mobile_get_autologin_key",
            &[("privatetoken", private)],
            false,
        )
        .await
    }
    pub async fn unread_notifications(&self, token: &str, user_id: i64) -> Result<u64, Error> {
        self.call(
            token,
            "message_popup_get_unread_popup_notification_count",
            &[("useridto", &user_id.to_string())],
            true,
        )
        .await
    }
    pub async fn unread_conversations(&self, token: &str, user_id: i64) -> Result<u64, Error> {
        self.call(
            token,
            "core_message_get_unread_conversations_count",
            &[("useridto", &user_id.to_string())],
            true,
        )
        .await
    }
}
fn decode_response<T: DeserializeOwned>(value: serde_json::Value) -> Result<T, Error> {
    if let Some(code) = value.get("errorcode").and_then(|c| c.as_str()) {
        return Err(match code {
            "invalidtoken" => Error::InvalidToken,
            "accessexception" | "nopermissions" => Error::AccessDenied,
            "autologinkeygenerationlockout" => Error::RateLimited,
            _ => Error::WebServices,
        });
    }
    serde_json::from_value(value).map_err(|_| Error::WebServices)
}
async fn backoff(attempt: u32) {
    // Non-secret jitter spreads clients without introducing a random dependency.
    let jitter = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_millis()
        % 200;
    tokio::time::sleep(std::time::Duration::from_millis(
        250 * (1 << attempt) + u64::from(jitter),
    ))
    .await;
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

#[cfg(test)]
mod contract_tests {
    use super::*;
    use wiremock::{
        matchers::{body_string_contains, method, path},
        Mock, MockServer, ResponseTemplate,
    };
    async fn client() -> (MockServer, Client) {
        let server = MockServer::start().await;
        // HTTP is confined to this test-only constructor; production always uses HTTPS.
        let client = Client {
            base: Url::parse(&format!("{}/", server.uri())).unwrap(),
            http: reqwest::Client::new(),
            permits: tokio::sync::Semaphore::new(4),
        };
        (server, client)
    }
    #[tokio::test]
    async fn site_info_posts_token_in_body_and_decodes_response() {
        let (server, client) = client().await;
        Mock::given(method("POST")).and(path("/webservice/rest/server.php"))
            .and(body_string_contains("wstoken=test-token"))
            .and(body_string_contains("wsfunction=core_webservice_get_site_info"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"userid":42,"fullname":"Test Learner","sitename":"Test Moodle","functions":[]})))
            .expect(1).mount(&server).await;
        assert_eq!(client.site_info("test-token").await.unwrap().userid, 42);
        assert!(!server.received_requests().await.unwrap()[0]
            .url
            .as_str()
            .contains("test-token"));
    }
    #[tokio::test]
    async fn invalid_token_is_typed_and_not_retried_or_exposed() {
        let (server, client) = client().await;
        Mock::given(method("POST")).respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"exception":"moodle_exception","errorcode":"invalidtoken","message":"private server detail test-token"}))).expect(1).mount(&server).await;
        let error = client.site_info("test-token").await.err().unwrap();
        assert!(matches!(error, Error::InvalidToken));
        assert!(!error.to_string().contains("test-token"));
    }
    #[tokio::test]
    async fn autologin_key_generation_is_not_retried() {
        let (server, client) = client().await;
        Mock::given(method("POST"))
            .and(body_string_contains("privatetoken=test-private"))
            .respond_with(ResponseTemplate::new(503))
            .expect(1)
            .mount(&server)
            .await;
        assert!(client
            .autologin_key("test-token", "test-private")
            .await
            .is_err());
    }
    #[tokio::test]
    async fn transient_read_errors_have_bounded_retries() {
        let (server, client) = client().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(503))
            .expect(3)
            .mount(&server)
            .await;
        assert!(matches!(
            client.site_info("test-token").await,
            Err(Error::Network)
        ));
    }
    #[tokio::test]
    async fn public_config_decodes_stock_ajax_envelope() {
        let (server, client) = client().await;
        Mock::given(method("POST")).and(path("/lib/ajax/service.php"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([{"error":false,"data":{"wwwroot":"https://moodle.example.org","sitename":"Example","typeoflogin":3,"launchurl":"https://moodle.example.org/admin/tool/mobile/launch.php"}}])))
            .expect(1).mount(&server).await;
        assert_eq!(client.public_config().await.unwrap().sitename, "Example");
    }
    #[test]
    fn autologin_refuses_origin_path_and_query_injection() {
        let base = normalize_site("example.org/campus").unwrap();
        for target in [
            "https://evil.org/admin/tool/mobile/autologin.php",
            "https://example.org/other.php",
            "https://example.org/campus/admin/tool/mobile/autologin.php?urltogo=https://evil.org",
        ] {
            let key = AutologinKey {
                key: "test-key".into(),
                autologinurl: target.into(),
            };
            assert!(autologin_target(&base, &key, 42).is_err());
        }
        let key = AutologinKey {
            key: "test-key".into(),
            autologinurl: "https://example.org/campus/admin/tool/mobile/autologin.php".into(),
        };
        let url = autologin_target(&base, &key, 42).unwrap();
        assert!(url
            .query_pairs()
            .any(|(k, v)| k == "urltogo" && v == "https://example.org/campus/my/"));
    }
    #[test]
    fn moodle_errors_are_sanitized() {
        for (code, expected) in [
            ("accessexception", "permission"),
            ("autologinkeygenerationlockout", "limiting"),
        ] {
            let error = decode_response::<SiteInfo>(
                serde_json::json!({"errorcode":code,"message":"secret"}),
            )
            .err()
            .unwrap();
            assert!(error.to_string().contains(expected));
            assert!(!error.to_string().contains("secret"));
        }
    }
}

#[cfg(test)]
mod callback_limits {
    use super::*;
    #[test]
    fn oversized_callback_is_rejected_before_decoding() {
        let url = Url::parse(&format!("moodlemobile://token={}", "A".repeat(4096))).unwrap();
        assert!(matches!(
            parse_launch_token(&url, "https://example.org", "passport"),
            Err(Error::InvalidCallback)
        ));
    }
}
