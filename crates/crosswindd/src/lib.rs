//! The Crosswind system service: radio leases and the system D-Bus API.
//!
//! [`api`] is the API as clients see it, for `crosswind` and omdrop's hooks;
//! [`service`] is the service behind it.

pub mod api;
pub mod service;
