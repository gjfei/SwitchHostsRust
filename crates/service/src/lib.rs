pub mod client;

#[cfg(feature = "api")]
pub mod api;

#[cfg(feature = "scheduler")]
pub mod scheduler;

#[cfg(feature = "api")]
pub use api::{start_api, ApiHandle, HTTP_API_PORT};
pub use client::{
    build_client, fetch_url, fetch_url_with_client, ClientConfig, FetchError, MAX_RESPONSE_BYTES,
};

#[cfg(feature = "scheduler")]
pub use scheduler::{RefreshScheduler, SchedulerConfig};
