#[cfg(target_os = "linux")]
/// The target system's Io implementation
pub use crate::platform::linux::io::LinuxIo as TargetIo;
