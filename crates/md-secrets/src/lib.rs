//! Only this crate talks to the operating system credential store.
#[cfg(not(target_os = "macos"))]
mod legacy;
#[cfg(not(target_os = "macos"))]
pub use legacy::*;
#[cfg(target_os = "macos")]
mod vault;
#[cfg(target_os = "macos")]
pub use vault::*;

/// Supply the migration inventory before any credential consumer starts.
#[cfg(not(target_os = "macos"))]
pub fn configure_legacy_sites(_sites: impl IntoIterator<Item = String>) {}
