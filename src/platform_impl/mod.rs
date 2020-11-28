pub use self::platform_impl::*;

#[cfg(target_os = "windows")]
#[path = "windows/mod.rs"]
mod platform_impl;

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
#[path = "linux/mod.rs"]
mod platform_impl;
