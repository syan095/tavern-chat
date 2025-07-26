//! Contains logic that parses a string into proper event

use crate::common::*;
use tokio::sync::mpsc::Sender;

pub async fn parse_incoming_message(
    from: ChatTarget,
    message_raw: String,
    event_tx: Sender<Event>,
) -> ServerResult {
    println!("{:?}: {:?}", from, message_raw);
    if message_raw == "shutdown".to_owned() {
        let _ = event_tx.send(Event::Shutdown).await;
    }

    Ok(())
}
