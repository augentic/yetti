//! # Realtime Core
//!
//! Core modules for the Realtime platform.

pub mod api;
mod capabilities;
mod error;

#[cfg(target_arch = "wasm32")]
pub use qwasr_guest_macro::*;
pub use {anyhow, axum, bytes, http, http_body, tracing};
#[cfg(target_arch = "wasm32")]
pub use {
    wasip3, wit_bindgen, qwasr_wasi_http, qwasr_wasi_identity, qwasr_wasi_keyvalue,
    qwasr_wasi_messaging, qwasr_wasi_otel,
};

pub use crate::api::*;
pub use crate::capabilities::*;
pub use crate::error::*;

/// Checks required environment variables are set, panicking if any are
/// missing.
///
/// # Example
/// ```rust,ignore
/// qwasr_sdk::ensure_env!("API_KEY", "SOME_URL");
/// ```
#[macro_export]
macro_rules! ensure_env {
    ($($var:literal),+ $(,)?) => {
        {
            let mut missing = Vec::new();
            $(
                if std::env::var($var).is_err() {
                    missing.push($var);
                }
            )+

            if !missing.is_empty() {
                panic!("Missing required environment variables: {}", missing.join(", "));
            }
        }
    };
}
