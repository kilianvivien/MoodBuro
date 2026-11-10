//! Platform types and filesystem policy for desktop integration.
#[derive(Debug, Clone, Copy)]
pub enum Platform {
    MacOs,
    Windows,
    Linux,
}
pub mod downloads;
pub mod offline;
