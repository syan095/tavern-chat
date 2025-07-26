////////////////////////////////////////////////////////////////////////////////////////////////
//! This project simulates a Fantasy style Tavern.
//! A main server can be connected via TCP connections. Connected client can interact with
//! other clients and NPCs.
//!
//! This project is intended to be used to learn about tokio, channels and multi-threading in general.
//!
//! Roy Sirui Yang 2025
//!

use crate::server::TavernServer;

mod common;
mod npcs;
mod parser;
mod server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (mut server, _event_tx) = TavernServer::new();
    let handle = server.run();

    // Run until server exits.
    handle.await
}
