//! Calendar providers return credential-bearing feeds only to Rust. Never Serialize/Debug.
use crate::{Client, Error};
use serde::Deserialize;
use url::Url;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

/// Keep provider discovery here; an optional plugin provider can be added without
/// changing native handoff, consent, snapshot export, or local reminder semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    Moodle,
}
impl Provider {
    pub fn id(self) -> &'static str {
        match self {
            Self::Moodle => "moodle",
        }
    }
}
#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
struct ExportToken {
    token: String,
}
pub struct Feed {
    url: Zeroizing<String>,
    pub provider: Provider,
}
impl Feed {
    pub fn as_str(&self) -> &str {
        &self.url
    }
    fn moodle(base: &Url, user: i64, token: &str) -> Result<Self, Error> {
        if user <= 0 || token.len() != 40 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(Error::WebServices);
        }
        let mut url = base
            .join("calendar/export_execute.php")
            .map_err(|_| Error::InvalidUrl)?;
        url.query_pairs_mut()
            .append_pair("userid", &user.to_string())
            .append_pair("authtoken", token)
            .append_pair("preset_what", "all")
            .append_pair("preset_time", "recentupcoming");
        Ok(Self {
            url: Zeroizing::new(url.to_string()),
            provider: Provider::Moodle,
        })
    }
}
impl Client {
    pub async fn calendar_feed(&self, token: &str, expected_user: i64) -> Result<Feed, Error> {
        // Do not interpret a web-service token as a calendar credential.
        if self.site_info(token).await?.userid != expected_user {
            return Err(Error::AccessDenied);
        }
        let exported: ExportToken = self
            .call(token, "core_calendar_get_calendar_export_token", &[], false)
            .await?;
        Feed::moodle(&self.base, expected_user, &exported.token)
    }
    pub async fn probe_calendar(&self, feed: &Feed) -> Result<usize, Error> {
        let url = Url::parse(feed.as_str()).map_err(|_| Error::InvalidUrl)?;
        if url.scheme() != "https"
            || url.origin() != self.base.origin()
            || url.path()
                != self
                    .base
                    .join("calendar/export_execute.php")
                    .map_err(|_| Error::InvalidUrl)?
                    .path()
        {
            return Err(Error::InvalidUrl);
        }
        let http = reqwest::Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|_| Error::Network)?;
        probe(&http, url).await
    }
}
async fn probe(http: &reqwest::Client, url: Url) -> Result<usize, Error> {
    const MAX: u64 = 5 * 1024 * 1024;
    let mut response = http.get(url).send().await.map_err(|_| Error::Network)?;
    if !response.status().is_success() {
        return Err(Error::AccessDenied);
    }
    if response.content_length().is_some_and(|n| n > MAX)
        || !response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|mime| {
                mime.split(';')
                    .next()
                    .is_some_and(|m| m.trim().eq_ignore_ascii_case("text/calendar"))
            })
    {
        return Err(Error::WebServices);
    }
    let mut body = Zeroizing::new(Vec::new());
    while let Some(chunk) = response.chunk().await.map_err(|_| Error::Network)? {
        if body.len() + chunk.len() > MAX as usize {
            return Err(Error::WebServices);
        }
        body.extend_from_slice(&chunk);
    }
    let text = std::str::from_utf8(&body).map_err(|_| Error::WebServices)?;
    let text = text.trim_start_matches('\u{feff}').trim();
    if !text.starts_with("BEGIN:VCALENDAR")
        || !text.ends_with("END:VCALENDAR")
        || !text.lines().any(|l| l == "VERSION:2.0")
    {
        return Err(Error::WebServices);
    }
    Ok(text.lines().filter(|line| *line == "BEGIN:VEVENT").count())
}
#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        matchers::{body_string_contains, method, path},
        Mock, MockServer, ResponseTemplate,
    };
    #[test]
    fn stock_feed_is_fixed_https_scoped_and_uses_a_distinct_calendar_token() {
        let base = crate::normalize_site("https://example.org/campus/").unwrap();
        let secret = "a".repeat(40);
        let feed = Feed::moodle(&base, 42, &secret).unwrap();
        let u = Url::parse(feed.as_str()).unwrap();
        assert_eq!(u.path(), "/campus/calendar/export_execute.php");
        assert_eq!(
            u.query_pairs().find(|(k, _)| k == "authtoken").unwrap().1,
            secret
        );
        assert!(!feed.as_str().contains("wstoken"));
        assert_eq!(feed.provider.id(), "moodle");
        assert!(Feed::moodle(&base, 42, "web-service-token").is_err());
        assert!(Feed::moodle(&base, 0, &secret).is_err());
    }
    #[tokio::test]
    async fn token_api_is_posted_without_retries_and_identity_is_checked() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_string_contains(
                "wsfunction=core_webservice_get_site_info",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"userid":42,"fullname":"Learner","sitename":"Example"}),
            ))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(body_string_contains(
                "wsfunction=core_calendar_get_calendar_export_token",
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"token":"a".repeat(40),"warnings":[]})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let client = Client {
            base: Url::parse(&format!("{}/", server.uri())).unwrap(),
            http: reqwest::Client::new(),
            permits: tokio::sync::Semaphore::new(4),
        };
        let feed = client.calendar_feed("api-secret", 42).await.unwrap();
        assert!(!feed.as_str().contains("api-secret"));
        assert!(client.calendar_feed("api-secret", 43).await.is_err());
    }
    #[tokio::test]
    async fn probe_accepts_empty_calendars_and_rejects_redirects_login_pages_and_invalid_feeds() {
        let server = MockServer::start().await;
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        for (endpoint, response, success) in [
            (
                "/ok",
                ResponseTemplate::new(200).set_body_raw(
                    "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nEND:VCALENDAR\r\n",
                    "text/calendar",
                ),
                true,
            ),
            (
                "/login",
                ResponseTemplate::new(200).set_body_raw("login", "text/html"),
                false,
            ),
            (
                "/bad",
                ResponseTemplate::new(200).set_body_raw("Invalid authentication", "text/calendar"),
                false,
            ),
            (
                "/redirect",
                ResponseTemplate::new(302)
                    .insert_header("Location", "https://evil.example/calendar"),
                false,
            ),
            ("/revoked", ResponseTemplate::new(403), false),
        ] {
            Mock::given(path(endpoint))
                .respond_with(response)
                .mount(&server)
                .await;
            let result = probe(
                &http,
                Url::parse(&format!("{}{endpoint}?authtoken=private", server.uri())).unwrap(),
            )
            .await;
            assert_eq!(result.is_ok(), success);
            if let Err(e) = result {
                assert!(!e.to_string().contains("private"));
            }
        }
    }
}
