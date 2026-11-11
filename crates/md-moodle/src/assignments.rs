//! File-only, individual assignment drafts. Never calls a final-submission API.
use crate::{plain_text, Client, Error};
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use serde_json::Value;
pub const MAX_UPLOAD_BYTES: usize = 32 * 1024 * 1024;
#[derive(Clone, Deserialize)]
pub struct Assignment {
    pub id: i64,
    pub cmid: i64,
    pub course: i64,
    pub name: String,
    submissiondrafts: Option<i64>,
    teamsubmission: Option<i64>,
    nosubmissions: Option<i64>,
    #[serde(default)]
    configs: Vec<Config>,
}
#[derive(Clone, Deserialize)]
struct Config {
    plugin: String,
    subtype: String,
    name: String,
    value: String,
}
impl Assignment {
    pub fn supported(&self) -> bool {
        self.id > 0
            && self.cmid > 0
            && self.submissiondrafts == Some(1)
            && self.teamsubmission == Some(0)
            && self.nosubmissions == Some(0)
            && self.configs.iter().any(|c| {
                c.subtype == "assignsubmission"
                    && c.plugin == "file"
                    && c.name == "enabled"
                    && c.value == "1"
            })
            && !self.configs.iter().any(|c| {
                c.subtype == "assignsubmission"
                    && c.plugin != "file"
                    && c.name == "enabled"
                    && c.value == "1"
            })
    }
    pub fn allows_size(&self, bytes: usize) -> bool {
        bytes <= MAX_UPLOAD_BYTES
            && self
                .configs
                .iter()
                .find(|c| c.plugin == "file" && c.name == "maxsubmissionsizebytes")
                .and_then(|c| c.value.parse::<usize>().ok())
                .is_none_or(|limit| limit == 0 || bytes <= limit)
    }
}
#[derive(Deserialize)]
struct Assignments {
    courses: Vec<Course>,
}
#[derive(Deserialize)]
struct Course {
    id: i64,
    assignments: Vec<Assignment>,
}
/// Unknown, locked, submitted, team or nonempty work is deliberately not replaced.
pub fn empty_editable_status(status: &Value, user: i64) -> bool {
    let Some(last) = status.get("lastattempt") else {
        return false;
    };
    if last.get("canedit").and_then(Value::as_bool) != Some(true)
        || last.get("locked").and_then(Value::as_bool) != Some(false)
        || last.get("submissionsenabled").and_then(Value::as_bool) != Some(true)
    {
        return false;
    }
    if last.get("teamsubmission").is_some_and(|v| !v.is_null()) {
        return false;
    }
    let Some(sub) = last.get("submission").filter(|v| !v.is_null()) else {
        return true;
    };
    if sub.get("userid").and_then(Value::as_i64) != Some(user)
        || !matches!(
            sub.get("status").and_then(Value::as_str),
            Some("new" | "draft")
        )
    {
        return false;
    }
    let Some(plugins) = sub.get("plugins").and_then(Value::as_array) else {
        return false;
    };
    if !plugins.iter().any(|p| {
        p.get("type").and_then(Value::as_str) == Some("file")
            && p.get("fileareas").and_then(Value::as_array).is_some()
    }) {
        return false;
    }
    plugins.iter().all(|p| {
        let files = p
            .get("fileareas")
            .and_then(Value::as_array)
            .is_none_or(|areas| {
                areas.iter().all(|a| {
                    a.get("files")
                        .and_then(Value::as_array)
                        .is_some_and(Vec::is_empty)
                })
            });
        let text = p
            .get("editorfields")
            .and_then(Value::as_array)
            .is_none_or(|fields| {
                fields.iter().all(|f| {
                    f.get("text")
                        .and_then(Value::as_str)
                        .is_some_and(str::is_empty)
                })
            });
        files && text
    })
}
pub fn saved_draft_contains(status: &Value, user: i64, name: &str, bytes: usize) -> bool {
    let Some(sub) = status.pointer("/lastattempt/submission") else {
        return false;
    };
    sub.get("status").and_then(Value::as_str) == Some("draft")
        && sub.get("userid").and_then(Value::as_i64) == Some(user)
        && sub
            .get("plugins")
            .and_then(Value::as_array)
            .is_some_and(|plugins| {
                plugins.iter().any(|p| {
                    p.get("type").and_then(Value::as_str) == Some("file")
                        && p.get("fileareas")
                            .and_then(Value::as_array)
                            .is_some_and(|areas| {
                                areas.iter().any(|a| {
                                    a.get("files")
                                        .and_then(Value::as_array)
                                        .is_some_and(|files| {
                                            files.len() == 1
                                                && files[0].get("filename").and_then(Value::as_str)
                                                    == Some(name)
                                                && files[0].get("filesize").and_then(Value::as_u64)
                                                    == Some(bytes as u64)
                                        })
                                })
                            })
                })
            })
}
impl Client {
    pub async fn assignments(&self, token: &str, course: i64) -> Result<Vec<Assignment>, Error> {
        let result: Assignments = self
            .call(
                token,
                "mod_assign_get_assignments",
                &[("courseids[0]", &course.to_string())],
                true,
            )
            .await?;
        let items: Vec<_> = result
            .courses
            .into_iter()
            .filter(|c| c.id == course)
            .flat_map(|c| c.assignments)
            .filter(|a| a.course == course)
            .take(501)
            .map(|mut a| {
                a.name = plain_text(&a.name, 200);
                a
            })
            .collect();
        if items.len() > 500 {
            return Err(Error::WebServices);
        }
        Ok(items)
    }
    pub async fn assignment_status(&self, token: &str, assignment: i64) -> Result<Value, Error> {
        self.call(
            token,
            "mod_assign_get_submission_status",
            &[("assignid", &assignment.to_string())],
            true,
        )
        .await
    }
    pub async fn upload_draft_file(
        &self,
        token: &str,
        user: i64,
        name: &str,
        data: Vec<u8>,
    ) -> Result<i64, Error> {
        let endpoint = self
            .base
            .join("webservice/upload.php")
            .map_err(|_| Error::InvalidUrl)?;
        let http = reqwest::Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|_| Error::Network)?;
        upload(&http, endpoint, token, user, name, data).await
    }
    pub async fn save_file_draft(
        &self,
        token: &str,
        assignment: i64,
        item: i64,
    ) -> Result<(), Error> {
        // A redirect can replay a POST too. Writes use a dedicated no-redirect client.
        let _permit = self.permits.acquire().await.map_err(|_| Error::Network)?;
        let http = reqwest::Client::builder()
            .https_only(!cfg!(test))
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(30))
            .user_agent(concat!(
                "MoodBuro/",
                env!("CARGO_PKG_VERSION"),
                " MoodleMobile"
            ))
            .build()
            .map_err(|_| Error::Network)?;
        let writer = Client {
            base: self.base.clone(),
            http,
            permits: tokio::sync::Semaphore::new(1),
        };
        let warnings: Vec<Value> = writer
            .call(
                token,
                "mod_assign_save_submission",
                &[
                    ("assignmentid", &assignment.to_string()),
                    ("plugindata[files_filemanager]", &item.to_string()),
                ],
                false,
            )
            .await?;
        if warnings.is_empty() {
            Ok(())
        } else {
            Err(Error::WebServices)
        }
    }
}
async fn upload(
    http: &reqwest::Client,
    endpoint: url::Url,
    token: &str,
    user: i64,
    name: &str,
    data: Vec<u8>,
) -> Result<i64, Error> {
    let bytes = data.len();
    if bytes == 0
        || bytes > MAX_UPLOAD_BYTES
        || name.is_empty()
        || name.len() > 180
        || name
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        return Err(Error::WebServices);
    }
    let form = Form::new()
        .text("token", token.to_owned())
        .text("itemid", "0")
        .text("filepath", "/")
        .part("file_1", Part::bytes(data).file_name(name.to_owned()));
    let mut response = http
        .post(endpoint)
        .multipart(form)
        .send()
        .await
        .map_err(|_| Error::Network)?;
    if !response.status().is_success() {
        return Err(Error::Network);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| Error::Network)? {
        if body.len() + chunk.len() > 128 * 1024 {
            return Err(Error::WebServices);
        }
        body.extend_from_slice(&chunk);
    }
    let values: Vec<Value> = serde_json::from_slice(&body).map_err(|_| Error::WebServices)?;
    let file = values
        .first()
        .filter(|_| values.len() == 1)
        .ok_or(Error::WebServices)?;
    if file.get("error").is_some()
        || file.get("userid").and_then(Value::as_i64) != Some(user)
        || file.get("component").and_then(Value::as_str) != Some("user")
        || file.get("filearea").and_then(Value::as_str) != Some("draft")
        || file.get("filepath").and_then(Value::as_str) != Some("/")
        || file.get("filename").and_then(Value::as_str) != Some(name)
        || file.get("filesize").and_then(Value::as_u64) != Some(bytes as u64)
    {
        return Err(Error::WebServices);
    }
    file.get("itemid")
        .and_then(Value::as_i64)
        .filter(|n| *n > 0)
        .ok_or(Error::WebServices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::{
        matchers::{body_string_contains, method, path},
        Mock, MockServer, ResponseTemplate,
    };
    fn assignment() -> Value {
        json!({"id":9,"cmid":19,"course":2,"name":"Essay","submissiondrafts":1,"teamsubmission":0,"nosubmissions":0,"configs":[{"subtype":"assignsubmission","plugin":"file","name":"enabled","value":"1"},{"subtype":"assignsubmission","plugin":"file","name":"maxsubmissionsizebytes","value":"200"}]})
    }
    fn empty() -> Value {
        json!({"lastattempt":{"canedit":true,"locked":false,"submissionsenabled":true,"submission":{"userid":7,"status":"draft","plugins":[{"type":"file","fileareas":[{"area":"submission_files","files":[]}]}]}}})
    }
    #[test]
    fn only_explicit_individual_file_drafts_are_supported() {
        let base = assignment();
        let a: Assignment = serde_json::from_value(base.clone()).unwrap();
        assert!(a.supported());
        assert!(a.allows_size(200));
        assert!(!a.allows_size(201));
        for (field, value) in [
            ("submissiondrafts", json!(0)),
            ("teamsubmission", json!(1)),
            ("nosubmissions", json!(1)),
            ("submissiondrafts", Value::Null),
        ] {
            let mut v = base.clone();
            v[field] = value;
            assert!(!serde_json::from_value::<Assignment>(v).unwrap().supported());
        }
        let mut mixed = base;
        mixed["configs"].as_array_mut().unwrap().push(json!({"subtype":"assignsubmission","plugin":"onlinetext","name":"enabled","value":"1"}));
        assert!(!serde_json::from_value::<Assignment>(mixed)
            .unwrap()
            .supported());
    }
    #[test]
    fn unknown_nonempty_locked_submitted_or_other_users_work_is_not_replaced() {
        assert!(empty_editable_status(&empty(), 7));
        for (pointer, value) in [
            ("/lastattempt/canedit", json!(false)),
            ("/lastattempt/locked", json!(true)),
            ("/lastattempt/submissionsenabled", Value::Null),
            ("/lastattempt/submission/userid", json!(8)),
            ("/lastattempt/submission/status", json!("submitted")),
            (
                "/lastattempt/submission/plugins/0/fileareas/0/files",
                json!([{"filename":"existing.pdf"}]),
            ),
            ("/lastattempt/submission/plugins/0/fileareas", Value::Null),
        ] {
            let mut v = empty();
            *v.pointer_mut(pointer).unwrap() = value;
            assert!(!empty_editable_status(&v, 7), "{pointer}");
        }
        assert!(!empty_editable_status(&json!({}), 7));
        let mut team = empty();
        team["lastattempt"]["teamsubmission"] = json!({"status":"draft"});
        assert!(!empty_editable_status(&team, 7));
        let mut saved = empty();
        saved["lastattempt"]["submission"]["plugins"][0]["fileareas"][0]["files"] =
            json!([{"filename":"essay.pdf","filesize":5}]);
        assert!(saved_draft_contains(&saved, 7, "essay.pdf", 5));
        assert!(!saved_draft_contains(&saved, 8, "essay.pdf", 5));
        saved["lastattempt"]["submission"]["status"] = json!("submitted");
        assert!(!saved_draft_contains(&saved, 7, "essay.pdf", 5));
    }
    async fn fixture() -> (MockServer, Client) {
        let server = MockServer::start().await;
        let client = Client {
            base: url::Url::parse(&format!("{}/", server.uri())).unwrap(),
            http: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap(),
            permits: tokio::sync::Semaphore::new(4),
        };
        (server, client)
    }
    #[tokio::test]
    async fn upload_uses_body_credentials_and_checks_the_returned_owner_and_file() {
        let (server, client) = fixture().await;
        let good = json!([{"userid":7,"component":"user","filearea":"draft","filepath":"/","filename":"essay.pdf","filesize":5,"itemid":99}]);
        Mock::given(method("POST"))
            .and(path("/webservice/upload.php"))
            .and(body_string_contains("name=\"token\"\r\n\r\nfixture-secret"))
            .and(body_string_contains(
                "name=\"file_1\"; filename=\"essay.pdf\"",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(good.clone()))
            .expect(1)
            .mount(&server)
            .await;
        let endpoint = client.base.join("webservice/upload.php").unwrap();
        assert_eq!(
            upload(
                &client.http,
                endpoint.clone(),
                "fixture-secret",
                7,
                "essay.pdf",
                b"hello".to_vec()
            )
            .await
            .unwrap(),
            99
        );
        let requests = server.received_requests().await.unwrap();
        assert!(requests[0].url.query().is_none());
        server.reset().await;
        let mut wrong = good;
        wrong[0]["userid"] = json!(8);
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(wrong))
            .expect(1)
            .mount(&server)
            .await;
        assert!(upload(
            &client.http,
            endpoint,
            "fixture-secret",
            7,
            "essay.pdf",
            b"hello".to_vec()
        )
        .await
        .is_err());
    }
    #[tokio::test]
    async fn upload_redirects_and_save_failures_are_never_replayed() {
        let (server, client) = fixture().await;
        Mock::given(method("POST"))
            .and(path("/webservice/upload.php"))
            .respond_with(ResponseTemplate::new(302).insert_header("Location", "/redirect"))
            .expect(1)
            .mount(&server)
            .await;
        assert!(upload(
            &client.http,
            client.base.join("webservice/upload.php").unwrap(),
            "fixture-secret",
            7,
            "essay.pdf",
            b"hello".to_vec()
        )
        .await
        .is_err());
        Mock::given(method("POST"))
            .and(path("/webservice/rest/server.php"))
            .and(body_string_contains(
                "wsfunction=mod_assign_save_submission",
            ))
            .and(body_string_contains("assignmentid=9"))
            .and(body_string_contains("plugindata%5Bfiles_filemanager%5D=99"))
            .respond_with(ResponseTemplate::new(503))
            .expect(1)
            .mount(&server)
            .await;
        assert!(client
            .save_file_draft("fixture-secret", 9, 99)
            .await
            .is_err());
        assert_eq!(server.received_requests().await.unwrap().len(), 2);
    }
    #[tokio::test]
    async fn draft_save_refuses_redirects_instead_of_reposting() {
        let (server, client) = fixture().await;
        Mock::given(method("POST"))
            .and(path("/webservice/rest/server.php"))
            .respond_with(ResponseTemplate::new(307).insert_header("Location", "/other"))
            .expect(1)
            .mount(&server)
            .await;
        assert!(client
            .save_file_draft("fixture-secret", 9, 99)
            .await
            .is_err());
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
    #[tokio::test]
    async fn save_warnings_are_not_treated_as_success() {
        let (server, client) = fixture().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!([{"warningcode":"locked","message":"private detail"}])),
            )
            .expect(1)
            .mount(&server)
            .await;
        assert!(matches!(
            client.save_file_draft("fixture-secret", 9, 99).await,
            Err(Error::WebServices)
        ));
    }
}
