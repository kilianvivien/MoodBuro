//! OS adapters are implemented in W4, after the macOS notification spike.
#[derive(Debug, Clone, Copy)]
pub enum Platform {
    MacOs,
    Windows,
    Linux,
}
