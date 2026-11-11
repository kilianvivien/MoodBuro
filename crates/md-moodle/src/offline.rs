//! Plain-file downloads through Moodle's token-authenticated file endpoint.
use crate::{Client, Error};
use serde::Deserialize;
use std::{io::Write, path::Path};
use url::Url;

pub const MAX_FILE_BYTES: u64 = 128 * 1024 * 1024;
#[derive(Clone, Default, Deserialize)]
pub struct Content {
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub filepath: Option<String>,
    #[serde(default)]
    pub filesize: Option<u64>,
    #[serde(default)]
    pub fileurl: Option<String>,
}
/// Only the saved installation's web-service file script may receive credentials.
/// Reject unknown query fields rather than persisting or replaying signed URLs.
pub fn file_url(base: &Url, raw: &str) -> Option<Url> {
    let url = Url::parse(raw).ok()?;
    let prefix = format!("{}webservice/pluginfile.php", base.path());
    if url.scheme() != "https"
        || url.origin() != base.origin()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || !(url.path() == prefix || url.path().starts_with(&format!("{prefix}/")))
        || url
            .query_pairs()
            .any(|(key, _)| !matches!(key.as_ref(), "forcedownload" | "file"))
    {
        return None;
    }
    Some(url)
}
/// Why an in-memory fetch was refused. Never carries the URL or token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FetchError {
    /// The source is not this installation's web-service file script.
    Source,
    /// The answer exceeded the caller's byte limit (declared or streamed).
    TooLarge,
    /// HTML, JSON or a non-PDF body: usually a login or error page.
    NotPdf,
    /// The token was refused.
    Denied,
    Network,
}
impl FetchError {
    pub fn code(self) -> &'static str {
        match self {
            Self::Source => "SOURCE",
            Self::TooLarge => "TOO_LARGE",
            Self::NotPdf => "NOT_PDF",
            Self::Denied => "ACCESS_DENIED",
            Self::Network => "NETWORK",
        }
    }
}
impl Client {
    /// Fetch one course PDF into memory, at most `limit` bytes, for the
    /// timetable reader: same origin, HTTPS-only, no redirects, token appended
    /// here only. Nothing is written to disk.
    pub async fn fetch_course_pdf(
        &self,
        token: &str,
        raw: &str,
        limit: usize,
    ) -> Result<Vec<u8>, FetchError> {
        let mut url = file_url(&self.base, raw).ok_or(FetchError::Source)?;
        url.query_pairs_mut().append_pair("token", token);
        let http = reqwest::Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(60))
            .user_agent(concat!("MoodBuro/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| FetchError::Network)?;
        let _permit = self
            .permits
            .acquire()
            .await
            .map_err(|_| FetchError::Network)?;
        receive_pdf(&http, url, limit).await
    }
    pub async fn save_course_file(
        &self,
        token: &str,
        raw: &str,
        destination: &Path,
        mut progress: impl FnMut(u64),
    ) -> Result<u64, String> {
        let mut url = file_url(&self.base, raw).ok_or("This resource cannot be saved offline.")?;
        url.query_pairs_mut().append_pair("token", token);
        // Never follow a redirect carrying a token, even within the same origin.
        let http = reqwest::Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(180))
            .user_agent(concat!("MoodBuro/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| "Could not download this resource.")?;
        let _permit = self
            .permits
            .acquire()
            .await
            .map_err(|_| "Could not download this resource.")?;
        receive(&http, url, destination, &mut progress).await
    }
}
async fn receive_pdf(
    http: &reqwest::Client,
    url: Url,
    limit: usize,
) -> Result<Vec<u8>, FetchError> {
    let mut response = http
        .get(url)
        .send()
        .await
        .map_err(|_| FetchError::Network)?;
    match response.status().as_u16() {
        200 => {}
        401 | 403 => return Err(FetchError::Denied),
        _ => return Err(FetchError::Network),
    }
    if response
        .content_length()
        .is_some_and(|bytes| bytes > limit as u64)
    {
        return Err(FetchError::TooLarge);
    }
    let mime = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if !matches!(
        mime.as_str(),
        "application/pdf" | "application/octet-stream" | "application/x-pdf" | ""
    ) {
        return Err(FetchError::NotPdf);
    }
    let mut body = Vec::new();
    while let Some(bytes) = response.chunk().await.map_err(|_| FetchError::Network)? {
        // Enforced while streaming: Content-Length may be absent or wrong.
        if body.len() + bytes.len() > limit {
            return Err(FetchError::TooLarge);
        }
        body.extend_from_slice(&bytes);
    }
    if !body.starts_with(b"%PDF-") {
        return Err(FetchError::NotPdf);
    }
    Ok(body)
}
async fn receive(
    http: &reqwest::Client,
    url: Url,
    destination: &Path,
    progress: &mut impl FnMut(u64),
) -> Result<u64, String> {
    receive_limited(http, url, destination, progress, MAX_FILE_BYTES).await
}
async fn receive_limited(
    http: &reqwest::Client,
    url: Url,
    destination: &Path,
    progress: &mut impl FnMut(u64),
    limit: u64,
) -> Result<u64, String> {
    let mut response = http
        .get(url)
        .send()
        .await
        .map_err(|_| "Could not download this resource.")?;
    if response.status() == reqwest::StatusCode::UNAUTHORIZED
        || response.status() == reqwest::StatusCode::FORBIDDEN
    {
        return Err(Error::AccessDenied.to_string());
    }
    if response.status().as_u16() != 200 {
        return Err("Could not download this resource.".into());
    }
    if response.content_length().is_some_and(|bytes| bytes > limit) {
        return Err("This resource exceeds the 128 MB offline limit.".into());
    }
    // HTML and JSON are commonly login/error pages. This slice supports plain files,
    // not downloaded web pages, package manifests or HTML course mirroring.
    let mime = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if matches!(
        mime.as_str(),
        "text/html" | "application/xhtml+xml" | "application/json"
    ) {
        return Err("This resource cannot be saved offline.".into());
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|_| "Could not save this resource.")?;
    let mut received = 0_u64;
    while let Some(bytes) = response
        .chunk()
        .await
        .map_err(|_| "Could not download this resource.")?
    {
        received = received
            .checked_add(bytes.len() as u64)
            .filter(|n| *n <= limit)
            .ok_or("This resource exceeds the 128 MB offline limit.")?;
        file.write_all(&bytes)
            .map_err(|_| "Could not save this resource.")?;
        progress(received);
    }
    file.sync_all()
        .map_err(|_| "Could not save this resource.")?;
    Ok(received)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};
    #[test]
    fn token_destinations_are_bound_to_the_installation() {
        let base = Url::parse("https://example.org/campus/").unwrap();
        assert!(file_url(&base, "https://example.org/campus/webservice/pluginfile.php/42/mod_resource/content/1/a.pdf?forcedownload=1").is_some());
        for raw in [
            "https://evil.org/campus/webservice/pluginfile.php/1",
            "https://example.org/other/webservice/pluginfile.php/1",
            "https://example.org/campus/pluginfile.php/1",
            "https://example.org/campus/webservice/pluginfile.php/1?token=secret",
            "https://example.org/campus/webservice/pluginfile.php.evil/1",
            "https://user@example.org/campus/webservice/pluginfile.php/1",
            "https://example.org/campus/webservice/pluginfile.php/1#secret",
        ] {
            assert!(file_url(&base, raw).is_none());
        }
    }
    #[tokio::test]
    async fn transfer_refuses_redirects_errors_large_files_and_never_overwrites() {
        let server = MockServer::start().await;
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let root = std::env::temp_dir().join(format!("moodburo-http-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("notes.txt");
        Mock::given(path("/ok"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("plain file")
                    .insert_header("Content-Type", "text/plain"),
            )
            .mount(&server)
            .await;
        let url =
            |path: &str| Url::parse(&format!("{}{path}?token=test-secret", server.uri())).unwrap();
        assert_eq!(
            receive(&http, url("/ok"), &file, &mut |_| {})
                .await
                .unwrap(),
            10
        );
        assert!(receive(&http, url("/ok"), &file, &mut |_| {})
            .await
            .is_err());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "plain file");
        for (endpoint, response) in [
            (
                "/redirect",
                ResponseTemplate::new(302)
                    .insert_header("Location", format!("{}/ok", server.uri())),
            ),
            ("/denied", ResponseTemplate::new(403)),
            (
                "/login",
                ResponseTemplate::new(200).set_body_raw("login", "text/html"),
            ),
        ] {
            Mock::given(path(endpoint))
                .respond_with(response)
                .mount(&server)
                .await;
            let error = receive(&http, url(endpoint), &root.join("refused"), &mut |_| {})
                .await
                .unwrap_err();
            assert!(!error.contains("test-secret"));
            assert!(!root.join("refused").exists());
        }
        assert!(
            receive_limited(&http, url("/ok"), &root.join("too-large"), &mut |_| {}, 9)
                .await
                .is_err()
        );
        assert!(!root.join("too-large").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn in_memory_pdf_fetch_is_bounded_and_refuses_pages_and_redirects() {
        let server = MockServer::start().await;
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let pdf = |n: usize| {
            let mut body = b"%PDF-1.7\n".to_vec();
            body.resize(n, b'x');
            body
        };
        for (endpoint, response) in [
            (
                "/exact",
                ResponseTemplate::new(200).set_body_raw(pdf(64), "application/pdf"),
            ),
            (
                "/over",
                ResponseTemplate::new(200).set_body_raw(pdf(65), "application/pdf"),
            ),
            (
                "/login",
                ResponseTemplate::new(200).set_body_raw("<html>login</html>", "text/html"),
            ),
            (
                "/disguised",
                ResponseTemplate::new(200).set_body_raw("<html>login</html>", "application/pdf"),
            ),
            ("/denied", ResponseTemplate::new(403)),
            (
                "/redirect",
                ResponseTemplate::new(302)
                    .insert_header("Location", format!("{}/exact", server.uri())),
            ),
        ] {
            Mock::given(path(endpoint))
                .respond_with(response)
                .mount(&server)
                .await;
        }
        let url = |p: &str| Url::parse(&format!("{}{p}?token=test-secret", server.uri())).unwrap();
        assert_eq!(
            receive_pdf(&http, url("/exact"), 64).await.unwrap().len(),
            64
        );
        assert_eq!(
            receive_pdf(&http, url("/over"), 64).await.unwrap_err(),
            FetchError::TooLarge
        );
        assert_eq!(
            receive_pdf(&http, url("/login"), 64).await.unwrap_err(),
            FetchError::NotPdf
        );
        assert_eq!(
            receive_pdf(&http, url("/disguised"), 64).await.unwrap_err(),
            FetchError::NotPdf
        );
        assert_eq!(
            receive_pdf(&http, url("/denied"), 64).await.unwrap_err(),
            FetchError::Denied
        );
        assert_eq!(
            receive_pdf(&http, url("/redirect"), 64).await.unwrap_err(),
            FetchError::Network
        );
    }
    #[tokio::test]
    async fn in_memory_fetch_enforces_the_limit_without_content_length() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 1024];
            let _ = socket.read(&mut request).await;
            // HTTP/1.0 style: no Content-Length, body ends at close.
            let mut answer = b"HTTP/1.1 200 OK\r\nContent-Type: application/pdf\r\nConnection: close\r\n\r\n%PDF-".to_vec();
            answer.extend(std::iter::repeat_n(b'x', 200));
            let _ = socket.write_all(&answer).await;
        });
        let http = reqwest::Client::new();
        let url = Url::parse(&format!("http://{address}/stream")).unwrap();
        assert_eq!(
            receive_pdf(&http, url, 100).await.unwrap_err(),
            FetchError::TooLarge
        );
    }
}
