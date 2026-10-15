//! Cliente do protocolo do Rojo (versão 5). Ver `docs/rojo-protocol.md`.

pub mod client;
pub mod error;
pub mod session;
pub mod sync;
pub mod types;

pub use client::{DEFAULT_PORT, RojoClient, Snapshot};
pub use error::{Error, Result, SUPPORTED_PROTOCOL_VERSION};
pub use session::{Poll, Session};
