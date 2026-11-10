//! Only this crate talks to the operating system credential store.
use zeroize::Zeroizing;
const SERVICE: &str = "app.moodburo";
#[derive(Clone, Copy)]
pub enum Kind {
    Token,
    Private,
}
fn entry(site: &str, kind: Kind) -> Result<keyring::Entry, keyring::Error> {
    let kind = match kind {
        Kind::Token => "token",
        Kind::Private => "private",
    };
    keyring::Entry::new(SERVICE, &format!("site:{site}:{kind}"))
}
pub fn put(site: &str, kind: Kind, value: &str) -> Result<(), keyring::Error> {
    entry(site, kind)?.set_password(value)
}
pub fn get(site: &str, kind: Kind) -> Result<Zeroizing<String>, keyring::Error> {
    entry(site, kind)?.get_password().map(Zeroizing::new)
}
pub fn delete_site(site: &str) -> Result<(), keyring::Error> {
    for kind in [Kind::Token, Kind::Private] {
        match entry(site, kind)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

pub fn delete(site: &str, kind: Kind) -> Result<(), keyring::Error> {
    match entry(site, kind)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(error),
    }
}

/// Distinguish absent credentials from a locked/unavailable credential store.
pub fn get_optional(site: &str, kind: Kind) -> Result<Option<Zeroizing<String>>, keyring::Error> {
    match get(site, kind) {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error),
    }
}

/// Bring-your-own AI provider keys, one entry per provider. Never site-scoped.
fn provider_entry(provider: &str) -> Result<keyring::Entry, keyring::Error> {
    keyring::Entry::new(SERVICE, &format!("ai:{provider}:key"))
}
pub fn provider_put(provider: &str, value: &str) -> Result<(), keyring::Error> {
    provider_entry(provider)?.set_password(value)
}
pub fn provider_get(provider: &str) -> Result<Option<Zeroizing<String>>, keyring::Error> {
    match provider_entry(provider)?.get_password() {
        Ok(value) => Ok(Some(Zeroizing::new(value))),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error),
    }
}
pub fn provider_delete(provider: &str) -> Result<(), keyring::Error> {
    match provider_entry(provider)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(error),
    }
}
