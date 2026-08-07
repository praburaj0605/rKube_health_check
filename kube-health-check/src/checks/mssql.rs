//! MSSQL readiness check via TCP connectivity.
//!
//! A full TDS handshake client is intentionally avoided to keep the default
//! dependency graph light; use [`crate::checks::TcpCheck`] semantics against
//! the SQL Server port (1433) or wrap a custom check with your preferred driver.

use crate::checks::TcpCheck;

/// MSSQL dependency check (TCP to host:port).
pub type MssqlCheck = TcpCheck;

/// Helper to construct an MSSQL TCP check with default name.
pub fn mssql(addr: impl Into<String>) -> MssqlCheck {
    TcpCheck::new("mssql", addr)
}
