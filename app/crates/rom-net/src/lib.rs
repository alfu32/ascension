//! Network play (task 86, `app/docs/network.md`): the protocol both ends speak, the game
//! server (one match per process: lobby, one-time keys, seat tokens, a leader stepping at the
//! match speed and relaying every tick's journal), and the client side (a connection, a
//! follower that applies the journals, and a headless client that `rom client` and the tests
//! run).
//!
//! The server's clock only decides when the leader steps; nothing about time or randomness
//! reaches the simulation. Keys and tokens come from the operating system's randomness,
//! outside the match. People change the world only through orders the leader submits
//! (`app/docs/network.md`, rules 1 and 3).

pub mod client;
pub mod keys;
pub mod protocol;
pub mod report;
pub mod server;
pub mod state;
mod transport;
pub mod url;

pub use client::{
    ClientOptions, ClientReport, Connection, Credential, Follower, Incoming, Welcome, hello,
    run_client,
};
pub use protocol::{
    Channel, ClientMessage, HASH_EVERY, Lobby, MapSpec, MatchSetup, PROTOCOL, RefuseReason,
    SeatInfo, SeatKind, SeatState, ServerMessage, VERSION, WireCommand, WireEntry,
};
pub use server::{
    DisconnectPolicy, Limits, Log, MatchPlan, Operator, Server, ServerConfig, ServerHandle, Status,
};
pub use state::{KeyCounts, StateDb};
pub use url::ServerUrl;

/// Everything that can go wrong in network play.
#[derive(Debug, thiserror::Error)]
pub enum NetError {
    /// A socket or file failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The server's state database failed.
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
    /// A server address could not be read.
    #[error("bad server address {0}")]
    Url(String),
    /// The WebSocket failed.
    #[error("websocket: {0}")]
    Ws(String),
    /// TLS failed or is not built in.
    #[error("tls: {0}")]
    Tls(String),
    /// The operating system gave no random bytes.
    #[error("random bytes: {0}")]
    Random(String),
    /// The server refused to admit us.
    #[error("refused: {detail}")]
    Refused {
        /// Why.
        reason: RefuseReason,
        /// In words.
        detail: String,
    },
    /// The other side broke the protocol, or the connection ended.
    #[error("{0}")]
    Protocol(String),
    /// The match could not be built, saved or loaded.
    #[error("{0}")]
    Match(String),
}
