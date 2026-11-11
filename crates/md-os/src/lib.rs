//! Platform types and filesystem policy for desktop integration.
#[derive(Debug, Clone, Copy)]
pub enum Platform {
    MacOs,
    Windows,
    Linux,
}
pub mod downloads;
pub mod metadata;
pub mod offline;

/// Hash an immutable, bounded upload snapshot selected through the native picker/drop.
pub fn upload_hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
#[cfg(unix)]
pub fn no_follow_flag() -> i32 {
    libc::O_NOFOLLOW
}
