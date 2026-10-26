//! Touchpad plugin entry point; native input remains a host responsibility.

#![forbid(unsafe_code)]

#[cfg(target_arch = "wasm32")]
mod guest;

pub mod gestures;
