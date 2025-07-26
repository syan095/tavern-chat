//! Contains structs and enums used across the entire projects.
//! Structs and enum here should be simple. More complex types,
//! or types with more complex behavior should have their dedicated file.

use serde::{Deserialize, Serialize};
use std::time::Instant;
use thiserror::Error;
use tokio::net::{TcpStream, tcp::OwnedWriteHalf};

pub type UserId = u32;
pub type NpcId = u32;
pub type ServerResult = Result<(), ServerError>;

#[derive(Debug, Clone)]
pub struct Message {
    pub from: Option<ChatTarget>,
    pub to: ChatTarget,
    pub content: String,
    pub timestamp: Instant,
    pub tone: MessageTone,
}

impl Message {
    pub fn new(
        from: Option<ChatTarget>,
        to: ChatTarget,
        content: &str,
        tone: Option<MessageTone>,
    ) -> Self {
        Message {
            from,
            to,
            content: content.to_owned(),
            timestamp: Instant::now(),
            tone: tone.unwrap_or_default(),
        }
    }

    pub fn to_output(&self) -> String {
        format!(
            "{:?} {:?} {:?}: {:?}",
            self.timestamp,
            self.from,
            self.tone.clone(),
            self.content
        )
    }
}

#[derive(Debug)]
pub enum Event {
    NewClient {
        connection: TcpStream,
    },
    DisconnectClient {
        id: UserId,
    },
    ReceiveMessage {
        from: ChatTarget,
        message_raw: String,
    },
    BroadcastMessage {
        message: Message,
    },
    Shutdown,
}

#[derive(Debug, Error, Clone, Copy)]
pub enum ServerError {
    TcpConnectionFailed(UserId),
    InvalidMessageTarget(ChatTarget),
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerError::TcpConnectionFailed(id) => {
                write!(f, "TCP connection failed for user {}", id)
            }
            ServerError::InvalidMessageTarget(id) => {
                write!(f, "Invalid target: {:?}", id)
            }
        }
    }
}

#[derive(Debug)]
pub struct Client {
    pub id: UserId,
    pub send_tx: OwnedWriteHalf,
    pub context: ClientContext,
}

#[derive(Debug, Clone, Eq, PartialEq, Default, Copy)]
pub enum ChatTarget {
    #[default]
    Global,
    Client(UserId),
    Npc(NpcId),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
// Caches current State of a client
pub struct ClientContext {
    pub current_target: Option<ChatTarget>,
    pub tone: MessageTone,
}

/// The emotion that's paired with this message
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageTone {
    #[default]
    Said,
    Yelled,
    Laughed,
    Whispered,
}
