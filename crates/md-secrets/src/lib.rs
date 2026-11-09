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
