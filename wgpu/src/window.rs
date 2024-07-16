//! Display rendering results on windows.
pub mod compositor;
#[cfg(all(unix, not(target_os = "macos")))]
mod wayland;
#[cfg(all(unix, not(target_os = "macos")))]
mod x11;

pub use compositor::Compositor;
pub use wgpu::Surface;

#[cfg(all(unix, not(target_os = "macos")))]
use rustix::fs::{major, minor};
#[cfg(all(unix, not(target_os = "macos")))]
use std::{fs::File, io::Read, path::PathBuf};

#[cfg(all(unix, not(target_os = "macos")))]
fn ids_from_dev(dev: u64) -> Option<(u16, u16)> {
    let path = PathBuf::from(format!(
        "/sys/dev/char/{}:{}/device",
        major(dev),
        minor(dev)
    ));
    let vendor = {
        let path = path.join("vendor");
        let mut file = File::open(&path).ok()?;
        let mut contents = String::new();
        let _ = file.read_to_string(&mut contents).ok()?;
        u16::from_str_radix(contents.trim().trim_start_matches("0x"), 16)
            .ok()?
    };
    let device = {
        let path = path.join("device");
        let mut file = File::open(&path).ok()?;
        let mut contents = String::new();
        let _ = file.read_to_string(&mut contents).ok()?;
        u16::from_str_radix(contents.trim().trim_start_matches("0x"), 16)
            .ok()?
    };

    Some((vendor, device))
}
