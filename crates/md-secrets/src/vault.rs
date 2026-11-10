//! A single macOS Keychain item. Never expose serialized credentials over IPC.
use keyring::{Error, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Mutex};
use zeroize::{Zeroize, Zeroizing};
const SERVICE: &str = "app.moodburo";
const ACCOUNT: &str = "credentials-v1";

#[derive(Clone, Copy)]
pub enum Kind {
    Token,
    Private,
}
fn site_key(site: &str, kind: Kind) -> String {
    format!(
        "site:{site}:{}",
        match kind {
            Kind::Token => "token",
            Kind::Private => "private",
        }
    )
}
fn provider_key(provider: &str) -> String {
    format!("ai:{provider}:key")
}
fn invalid() -> Error {
    Error::Invalid(
        "credential vault".into(),
        "Unavailable or unsupported credential data.".into(),
    )
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Vault {
    version: u32,
    secrets: BTreeMap<String, String>,
    /// Durable cleanup journal: never reimport an old credential after a crash.
    cleanup: Vec<String>,
}
impl Default for Vault {
    fn default() -> Self {
        Self {
            version: 1,
            secrets: BTreeMap::new(),
            cleanup: Vec::new(),
        }
    }
}
impl Drop for Vault {
    fn drop(&mut self) {
        for value in self.secrets.values_mut() {
            value.zeroize();
        }
    }
}
trait Backend {
    fn read(&mut self, account: &str) -> Result<Option<Zeroizing<String>>>;
    fn write(&mut self, account: &str, value: &str) -> Result<()>;
    fn delete(&mut self, account: &str) -> Result<()>;
}
struct Keychain;
impl Backend for Keychain {
    fn read(&mut self, account: &str) -> Result<Option<Zeroizing<String>>> {
        match keyring::Entry::new(SERVICE, account)?.get_password() {
            Ok(value) => Ok(Some(Zeroizing::new(value))),
            Err(Error::NoEntry) => Ok(None),
            Err(e) => Err(e),
        }
    }
    fn write(&mut self, account: &str, value: &str) -> Result<()> {
        keyring::Entry::new(SERVICE, account)?.set_password(value)
    }
    fn delete(&mut self, account: &str) -> Result<()> {
        match keyring::Entry::new(SERVICE, account)?.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => Ok(()),
            Err(e) => Err(e),
        }
    }
}
struct Store<B> {
    backend: B,
    legacy: Vec<String>,
    cached: Option<Vault>,
}
impl<B: Backend> Store<B> {
    fn save(&mut self, vault: &Vault) -> Result<()> {
        let encoded = Zeroizing::new(serde_json::to_string(vault).map_err(|_| invalid())?);
        self.backend.write(ACCOUNT, &encoded)
    }
    fn load(&mut self) -> Result<()> {
        if self.cached.is_none() {
            let vault = match self.backend.read(ACCOUNT)? {
                Some(encoded) => {
                    let vault: Vault = serde_json::from_str(&encoded).map_err(|_| invalid())?;
                    if vault.version != 1
                        || vault.cleanup.iter().any(|name| {
                            !self.legacy.contains(name) || !vault.secrets.contains_key(name)
                        })
                    {
                        return Err(invalid());
                    }
                    vault
                }
                None => {
                    let mut vault = Vault::default();
                    for name in &self.legacy {
                        if let Some(value) = self.backend.read(name)? {
                            vault.secrets.insert(name.clone(), value.to_string());
                            vault.cleanup.push(name.clone());
                        }
                    }
                    // Commit all secrets before deleting any old item. Even an empty
                    // vault is retained as a migration marker and cannot resurrect keys.
                    self.save(&vault)?;
                    vault
                }
            };
            self.cached = Some(vault);
        }
        let current = self.cached.as_ref().expect("loaded");
        if !current.cleanup.is_empty() {
            let mut next = current.clone();
            for name in &next.cleanup {
                self.backend.delete(name)?;
            }
            next.cleanup.clear();
            self.save(&next)?;
            self.cached = Some(next);
        }
        Ok(())
    }
    fn get(&mut self, name: &str) -> Result<Option<Zeroizing<String>>> {
        self.load()?;
        Ok(self
            .cached
            .as_ref()
            .expect("loaded")
            .secrets
            .get(name)
            .cloned()
            .map(Zeroizing::new))
    }
    fn update(&mut self, changes: &[(String, Option<&str>)]) -> Result<()> {
        self.load()?;
        let mut next = self.cached.as_ref().expect("loaded").clone();
        for (name, value) in changes {
            if let Some(mut old) = next.secrets.remove(name) {
                old.zeroize();
            }
            if let Some(value) = value {
                next.secrets.insert(name.clone(), (*value).to_owned());
            }
        }
        // Failed persistence must leave the cache unchanged (including deletion).
        self.save(&next)?;
        self.cached = Some(next);
        Ok(())
    }
}
static STORE: Mutex<Store<Keychain>> = Mutex::new(Store {
    backend: Keychain,
    legacy: Vec::new(),
    cached: None,
});
fn store() -> Result<std::sync::MutexGuard<'static, Store<Keychain>>> {
    STORE.lock().map_err(|_| invalid())
}
/// Called before shell creation and background workers; does not unlock Keychain.
pub fn configure_legacy_sites(sites: impl IntoIterator<Item = String>) {
    let mut store = STORE.lock().expect("credential inventory");
    assert!(
        store.cached.is_none(),
        "credential inventory must precede consumers"
    );
    store.legacy = sites
        .into_iter()
        .flat_map(|site| [site_key(&site, Kind::Token), site_key(&site, Kind::Private)])
        .chain([provider_key("albert"), provider_key("mistral")])
        .collect();
}
pub fn put(site: &str, kind: Kind, value: &str) -> Result<()> {
    store()?.update(&[(site_key(site, kind), Some(value))])
}
pub fn get(site: &str, kind: Kind) -> Result<Zeroizing<String>> {
    get_optional(site, kind)?.ok_or(Error::NoEntry)
}
pub fn get_optional(site: &str, kind: Kind) -> Result<Option<Zeroizing<String>>> {
    store()?.get(&site_key(site, kind))
}
pub fn delete(site: &str, kind: Kind) -> Result<()> {
    store()?.update(&[(site_key(site, kind), None)])
}
pub fn delete_site(site: &str) -> Result<()> {
    store()?.update(&[
        (site_key(site, Kind::Token), None),
        (site_key(site, Kind::Private), None),
    ])
}
pub fn provider_put(provider: &str, value: &str) -> Result<()> {
    store()?.update(&[(provider_key(provider), Some(value))])
}
pub fn provider_get(provider: &str) -> Result<Option<Zeroizing<String>>> {
    store()?.get(&provider_key(provider))
}
pub fn provider_delete(provider: &str) -> Result<()> {
    store()?.update(&[(provider_key(provider), None)])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Fake {
        items: BTreeMap<String, String>,
        reads: usize,
        writes: usize,
        fail_read: Option<String>,
        fail_write: bool,
        fail_delete: bool,
    }
    impl Backend for Fake {
        fn read(&mut self, name: &str) -> Result<Option<Zeroizing<String>>> {
            self.reads += 1;
            if self.fail_read.as_deref() == Some(name) {
                return Err(invalid());
            }
            Ok(self.items.get(name).cloned().map(Zeroizing::new))
        }
        fn write(&mut self, name: &str, value: &str) -> Result<()> {
            self.writes += 1;
            if self.fail_write {
                return Err(invalid());
            }
            self.items.insert(name.into(), value.into());
            Ok(())
        }
        fn delete(&mut self, name: &str) -> Result<()> {
            if self.fail_delete {
                return Err(invalid());
            }
            self.items.remove(name);
            Ok(())
        }
    }
    fn fixture() -> Store<Fake> {
        let legacy = vec![
            site_key("a", Kind::Token),
            site_key("a", Kind::Private),
            site_key("b", Kind::Token),
            provider_key("albert"),
            provider_key("mistral"),
        ];
        Store {
            backend: Fake {
                items: legacy
                    .iter()
                    .map(|key| (key.clone(), format!("secret-{key}")))
                    .collect(),
                ..Default::default()
            },
            legacy,
            cached: None,
        }
    }
    #[test]
    fn migration_preserves_all_accounts_and_providers_then_reads_only_one_item() {
        let mut s = fixture();
        s.load().unwrap();
        assert_eq!(s.backend.items.len(), 1);
        assert_eq!(s.cached.as_ref().unwrap().secrets.len(), 5);
        s.cached = None;
        s.backend.reads = 0;
        for key in s.legacy.clone() {
            assert!(s.get(&key).unwrap().is_some());
        }
        assert_eq!(s.backend.reads, 1);
    }
    #[test]
    fn failed_reads_or_writes_never_delete_legacy_credentials() {
        let mut s = fixture();
        s.backend.fail_read = Some(s.legacy[1].clone());
        assert!(s.load().is_err());
        assert_eq!(s.backend.items.len(), 5);
        assert_eq!(s.backend.writes, 0);
        s.backend.fail_read = None;
        s.backend.fail_write = true;
        assert!(s.load().is_err());
        assert_eq!(s.backend.items.len(), 5);
        assert!(s.cached.is_none());
    }
    #[test]
    fn interrupted_cleanup_resumes_from_durable_vault_without_reimporting() {
        let mut s = fixture();
        s.backend.fail_delete = true;
        assert!(s.load().is_err());
        assert!(s.backend.items.contains_key(ACCOUNT));
        s.cached = None;
        s.backend.fail_delete = false;
        s.backend.fail_read = Some(s.legacy[0].clone());
        s.load().unwrap();
        assert_eq!(s.backend.items.len(), 1);
        assert_eq!(s.cached.as_ref().unwrap().secrets.len(), 5);
    }
    #[test]
    fn writes_are_transactional_and_removal_is_scoped_and_survives_restart() {
        let mut s = fixture();
        s.load().unwrap();
        let token = site_key("a", Kind::Token);
        s.backend.fail_write = true;
        assert!(s.update(&[(token.clone(), Some("changed"))]).is_err());
        assert_ne!(&**s.get(&token).unwrap().as_ref().unwrap(), "changed");
        assert!(s.update(&[(token.clone(), None)]).is_err());
        assert!(s.get(&token).unwrap().is_some());
        s.backend.fail_write = false;
        s.update(&[(token.clone(), None), (site_key("a", Kind::Private), None)])
            .unwrap();
        s.cached = None;
        assert!(s.get(&token).unwrap().is_none());
        assert!(s.get(&site_key("b", Kind::Token)).unwrap().is_some());
        assert!(s.get(&provider_key("albert")).unwrap().is_some());
    }
    #[test]
    fn corrupt_unknown_or_denied_vaults_are_never_replaced() {
        for value in [
            "not json",
            r#"{"version":2,"secrets":{},"cleanup":[]}"#,
            r#"{"version":1,"secrets":{},"cleanup":["credentials-v1"]}"#,
        ] {
            let mut s = fixture();
            s.backend.items.insert(ACCOUNT.into(), value.into());
            assert!(s.load().is_err());
            assert_eq!(s.backend.writes, 0);
            assert_eq!(s.backend.items.len(), 6);
        }
        let mut s = fixture();
        s.backend.fail_read = Some(ACCOUNT.into());
        assert!(s.load().is_err());
        assert_eq!(s.backend.writes, 0);
    }
    #[test]
    fn concurrent_consumers_share_one_unlock() {
        let mut s = fixture();
        s.load().unwrap();
        s.cached = None;
        s.backend.reads = 0;
        let s = std::sync::Arc::new(Mutex::new(s));
        let tasks: Vec<_> = (0..8)
            .map(|_| {
                let s = s.clone();
                std::thread::spawn(move || {
                    assert!(s
                        .lock()
                        .unwrap()
                        .get(&provider_key("albert"))
                        .unwrap()
                        .is_some());
                })
            })
            .collect();
        for task in tasks {
            task.join().unwrap();
        }
        assert_eq!(s.lock().unwrap().backend.reads, 1);
    }
}
