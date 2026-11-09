//! Moodle protocol and navigation policy. No dependency on Tauri.
pub mod deep_link;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{de::DeserializeOwned, Deserialize};
use url::Url;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("This Moodle link is invalid or does not match a saved organisation.")]
    InvalidDeepLink,
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
    if target.scheme() == "blob" {
        return match Url::parse(target.path()) {
            Ok(inner)
                if inner.scheme() == "https"
                    && inner.username().is_empty()
                    && inner.password().is_none()
                    && inner.origin() == site.origin() =>
            {
                Navigation::Allow
            }
            _ => Navigation::Block,
        };
    }
    if target.scheme() != "https" || !target.username().is_empty() || target.password().is_some() {
        return Navigation::Block;
    }
    if site.origin() == target.origin() {
        Navigation::Allow
    } else {
        Navigation::External
    }
}

/// Embedded frames (Genially, video players, H5P…) render HTTPS content in place.
/// Only top-level navigations may leave the app, so a frame never opens the browser.
/// Anything else falls back to the top-level policy.
pub fn embeddable(target: &Url) -> bool {
    target.scheme() == "https" && target.username().is_empty() && target.password().is_none()
}

/// Blank bootstrap pages and one-use authentication URLs are not useful history.
pub fn history_url_allowed(base: &Url, target: &Url) -> bool {
    navigation(base, target) == Navigation::Allow
        && (target.scheme() == "blob" || shareable_url(target))
        && ![
            "/admin/tool/mobile/autologin.php",
            "/admin/tool/mobile/launch.php",
            "/login/token.php",
        ]
        .iter()
        .any(|path| target.path().ends_with(path))
}

/// Never send authentication material to another application through a URL.
pub fn shareable_url(url: &Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && !url.query_pairs().any(|(key, _)| {
            matches!(
                key.to_ascii_lowercase().as_str(),
                "key" | "token" | "wstoken" | "privatetoken" | "sesskey" | "passport"
            )
        })
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
    let dashboard = base.join("my/").map_err(|_| Error::InvalidUrl)?;
    autologin_target_to(base, login, user_id, &dashboard)
}

pub fn autologin_target_to(
    base: &Url,
    login: &AutologinKey,
    user_id: i64,
    destination: &Url,
) -> Result<Url, Error> {
    if navigation(base, destination) != Navigation::Allow
        || !destination.path().starts_with(base.path())
    {
        return Err(Error::InvalidUrl);
    }
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
        .append_pair("urltogo", destination.as_str());
    Ok(target)
}

/// Stock message_popup response. Full HTML bodies are intentionally not retained.
#[derive(Clone, Deserialize)]
pub struct PopupNotification {
    pub id: i64,
    pub subject: String,
    pub contexturl: Option<String>,
    pub timecreated: i64,
    pub read: bool,
}
#[derive(Deserialize)]
pub struct PopupNotifications {
    pub notifications: Vec<PopupNotification>,
}

/// Enrolled-course metadata; only the identifiers and names are retained.
#[derive(Clone, Deserialize)]
pub struct Course {
    pub id: i64,
    pub fullname: String,
    #[serde(default)]
    pub shortname: String,
}
#[derive(Clone, Deserialize)]
pub struct EventCourse {
    pub id: i64,
    #[serde(default)]
    pub fullname: String,
}
/// Upcoming calendar entry. Server-supplied URLs and descriptions are intentionally dropped.
#[derive(Clone, Deserialize)]
pub struct CalendarEvent {
    pub id: i64,
    pub name: String,
    pub timestart: i64,
    #[serde(default)]
    pub timeduration: i64,
    #[serde(default)]
    pub eventtype: String,
    #[serde(default)]
    pub course: Option<EventCourse>,
}
#[derive(Deserialize)]
pub struct UpcomingEvents {
    pub events: Vec<CalendarEvent>,
}

/// Fixed Moodle destinations the shell may open. Built locally from the saved base URL;
/// a server-supplied link is never followed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Notifications,
    Messages,
    Calendar,
    CalendarDay(i64),
    Course(i64),
}
pub fn page_url(base: &Url, page: Page) -> Result<Url, Error> {
    let (path, query) = match page {
        Page::Notifications => ("message/output/popup/notifications.php", None),
        Page::Messages => ("message/index.php", None),
        Page::Calendar => ("calendar/view.php", Some("view=upcoming".to_owned())),
        Page::CalendarDay(time) if time >= 0 => {
            ("calendar/view.php", Some(format!("view=day&time={time}")))
        }
        Page::Course(id) if id > 0 => ("course/view.php", Some(format!("id={id}"))),
        _ => return Err(Error::InvalidUrl),
    };
    let mut url = base.join(path).map_err(|_| Error::InvalidUrl)?;
    url.set_query(query.as_deref());
    Ok(url)
}

/// Moodle formats names as HTML. Keep bounded plain text: no tags, entities or control
/// characters reach SQLite, the shell or native menus.
pub fn plain_text(input: &str, max: usize) -> String {
    let mut text = String::with_capacity(input.len().min(max * 4));
    let mut in_tag = false;
    for c in input.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if in_tag => {}
            c if c.is_control() => text.push(' '),
            c => text.push(c),
        }
    }
    for (entity, value) in [
        ("&nbsp;", " "),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&#039;", "'"),
        ("&#39;", "'"),
        ("&amp;", "&"),
    ] {
        text = text.replace(entity, value);
    }
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max)
        .collect()
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
    pub async fn popup_notifications(
        &self,
        token: &str,
        user_id: i64,
    ) -> Result<PopupNotifications, Error> {
        self.call(
            token,
            "message_popup_get_popup_notifications",
            &[
                ("useridto", &user_id.to_string()),
                ("newestfirst", "1"),
                ("limit", "20"),
                ("offset", "0"),
            ],
            true,
        )
        .await
    }
    pub async fn mark_notification_read(
        &self,
        token: &str,
        notification_id: i64,
    ) -> Result<(), Error> {
        let _: serde_json::Value = self
            .call(
                token,
                "core_message_mark_notification_read",
                &[("notificationid", &notification_id.to_string())],
                false,
            )
            .await?;
        Ok(())
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
    pub async fn mark_all_notifications_read(
        &self,
        token: &str,
        user_id: i64,
    ) -> Result<(), Error> {
        let _: serde_json::Value = self
            .call(
                token,
                "core_message_mark_all_notifications_as_read",
                &[("useridto", &user_id.to_string())],
                false,
            )
            .await?;
        Ok(())
    }
    pub async fn courses(&self, token: &str, user_id: i64) -> Result<Vec<Course>, Error> {
        self.call(
            token,
            "core_enrol_get_users_courses",
            &[("userid", &user_id.to_string()), ("returnusercount", "0")],
            true,
        )
        .await
    }
    pub async fn upcoming_events(&self, token: &str) -> Result<Vec<CalendarEvent>, Error> {
        let view: UpcomingEvents = self
            .call(token, "core_calendar_get_calendar_upcoming_view", &[], true)
            .await?;
        Ok(view.events)
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
    fn frames_embed_https_in_place_and_nothing_else() {
        for target in [
            "https://view.genially.com/6262b06babb7e60011feced0",
            "https://www.youtube-nocookie.com/embed/x",
            "https://example.org/mod/h5pactivity/view.php?id=1",
        ] {
            assert!(embeddable(&Url::parse(target).unwrap()), "{target}");
        }
        for target in [
            "http://view.genially.com/",
            "https://user:pass@example.org/",
            "javascript:alert(1)",
            "file:///tmp/a",
            "about:blank",
            "data:text/html,x",
        ] {
            assert!(!embeddable(&Url::parse(target).unwrap()), "{target}");
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
    async fn notifications_and_counts_match_stock_moodle_shapes() {
        let (server, client) = client().await;
        for (function, response) in [
            (
                "message_popup_get_unread_popup_notification_count",
                serde_json::json!(3),
            ),
            (
                "core_message_get_unread_conversations_count",
                serde_json::json!(2),
            ),
            (
                "message_popup_get_popup_notifications",
                serde_json::json!({"notifications":[{"id":12,"subject":"Course update","contexturl":"https://example.org/course/view.php?id=2","timecreated":123,"read":false}],"unreadcount":3}),
            ),
        ] {
            Mock::given(method("POST"))
                .and(body_string_contains(format!("wsfunction={function}")))
                .and(body_string_contains("useridto=42"))
                .respond_with(ResponseTemplate::new(200).set_body_json(response))
                .expect(1)
                .mount(&server)
                .await;
        }
        assert_eq!(client.unread_notifications("test", 42).await.unwrap(), 3);
        assert_eq!(client.unread_conversations("test", 42).await.unwrap(), 2);
        let items = client.popup_notifications("test", 42).await.unwrap();
        assert_eq!(items.notifications[0].id, 12);
        assert!(!items.notifications[0].read);
        let requests = server.received_requests().await.unwrap();
        let body = String::from_utf8_lossy(&requests[2].body);
        assert!(body.contains("limit=20") && body.contains("newestfirst=1"));
    }
    #[tokio::test]
    async fn courses_events_and_mark_all_match_stock_moodle_shapes() {
        let (server, client) = client().await;
        for (function, arg, response) in [
            (
                "core_enrol_get_users_courses",
                "userid=42",
                serde_json::json!([{"id":7,"fullname":"Public Law &amp; Policy","shortname":"LAW1","summary":"<p>ignored</p>"},{"id":8,"fullname":"Economics"}]),
            ),
            (
                "core_calendar_get_calendar_upcoming_view",
                "moodlewsrestformat=json",
                serde_json::json!({"events":[{"id":3,"name":"Essay due","timestart":1800000000,"timeduration":0,"eventtype":"due","course":{"id":7,"fullname":"Public Law"},"url":"https://example.org/mod/assign/view.php?id=9"},{"id":4,"name":"Personal note","timestart":1800003600}]}),
            ),
            (
                "core_message_mark_all_notifications_as_read",
                "useridto=42",
                serde_json::json!(true),
            ),
        ] {
            Mock::given(method("POST"))
                .and(body_string_contains(format!("wsfunction={function}")))
                .and(body_string_contains(arg))
                .respond_with(ResponseTemplate::new(200).set_body_json(response))
                .expect(1)
                .mount(&server)
                .await;
        }
        let courses = client.courses("test", 42).await.unwrap();
        assert_eq!(courses.len(), 2);
        assert_eq!(plain_text(&courses[0].fullname, 64), "Public Law & Policy");
        assert_eq!(courses[1].shortname, "");
        let events = client.upcoming_events("test").await.unwrap();
        assert_eq!(events[0].course.as_ref().unwrap().id, 7);
        assert!(events[1].course.is_none() && events[1].eventtype.is_empty());
        client
            .mark_all_notifications_read("test", 42)
            .await
            .unwrap();
    }
    #[tokio::test]
    async fn denied_course_listing_is_typed() {
        let (server, client) = client().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"exception":"x","errorcode":"accessexception","message":"m"}),
            ))
            .mount(&server)
            .await;
        assert!(matches!(
            client.courses("test", 42).await,
            Err(Error::AccessDenied)
        ));
    }
    #[test]
    fn pages_are_fixed_and_names_are_plain_bounded_text() {
        let base = Url::parse("https://example.org/moodle/").unwrap();
        assert_eq!(
            page_url(&base, Page::Course(12)).unwrap().as_str(),
            "https://example.org/moodle/course/view.php?id=12"
        );
        assert_eq!(
            page_url(&base, Page::CalendarDay(1800000000))
                .unwrap()
                .as_str(),
            "https://example.org/moodle/calendar/view.php?view=day&time=1800000000"
        );
        assert!(page_url(&base, Page::Notifications)
            .unwrap()
            .path()
            .ends_with("/moodle/message/output/popup/notifications.php"));
        assert!(page_url(&base, Page::Course(0)).is_err());
        assert!(page_url(&base, Page::CalendarDay(-1)).is_err());
        for page in [Page::Messages, Page::Calendar, Page::Course(3)] {
            assert_eq!(
                navigation(&base, &page_url(&base, page).unwrap()),
                Navigation::Allow
            );
        }
        assert_eq!(
            plain_text(
                "<span lang=\"fr\">Droit</span>\n &amp; <b>Soci&#039;été</b>",
                64
            ),
            "Droit & Soci'été"
        );
        assert_eq!(plain_text("abcdef", 3), "abc");
    }
    #[tokio::test]
    async fn mark_read_is_not_retried() {
        let (server, client) = client().await;
        Mock::given(method("POST"))
            .and(body_string_contains(
                "wsfunction=core_message_mark_notification_read",
            ))
            .and(body_string_contains("notificationid=12"))
            .respond_with(ResponseTemplate::new(503))
            .expect(1)
            .mount(&server)
            .await;
        assert!(client.mark_notification_read("test", 12).await.is_err());
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

#[cfg(test)]
mod file_navigation_tests {
    use super::*;
    #[test]
    fn blob_documents_keep_their_site_boundary_and_external_links_drop_secrets() {
        let base = normalize_site("example.org").unwrap();
        assert_eq!(
            navigation(
                &base,
                &Url::parse("blob:https://example.org/document").unwrap()
            ),
            Navigation::Allow
        );
        for target in [
            "blob:https://other.example/document",
            "blob:null/document",
            "blob:https://user:pass@example.org/document",
        ] {
            assert_eq!(
                navigation(&base, &Url::parse(target).unwrap()),
                Navigation::Block
            );
        }
        for target in [
            "https://other.example/?%6bey=secret",
            "https://other.example/?TOKEN=secret",
            "https://user:pass@example.org/",
            "blob:https://example.org/document",
        ] {
            assert!(!shareable_url(&Url::parse(target).unwrap()));
        }
        assert!(shareable_url(
            &Url::parse("https://other.example/course?id=4").unwrap()
        ));
        for target in [
            "about:blank",
            "https://example.org/admin/tool/mobile/autologin.php?key=secret",
            "https://example.org/admin/tool/mobile/launch.php",
        ] {
            assert!(!history_url_allowed(&base, &Url::parse(target).unwrap()));
        }
        assert!(history_url_allowed(
            &base,
            &base.join("course/view.php?id=4").unwrap()
        ));
    }
}
