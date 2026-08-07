//! MongoDB readiness check via TCP connectivity.
//!
//! Avoids pulling the heavy `mongodb` driver (and its DNS/JNI graph). For a
//! deeper `ping` command check, register a [`crate::checks::CustomCheck`] using
//! your application's existing client.

use crate::checks::TcpCheck;

/// MongoDB dependency check (TCP to `host:port`, default 27017).
pub type MongoCheck = TcpCheck;

/// Helper to construct a MongoDB TCP check with the default name.
pub fn mongodb(addr: impl Into<String>) -> MongoCheck {
    TcpCheck::new("mongodb", addr)
}
