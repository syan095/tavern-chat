//! Contains structs and enums used across the entire projects.
//! Structs and enum here should be simple. More complex types,
//! or types with more complex behavior should have their dedicated file.

use chrono::{DateTime, Local};
use std::{fmt::Display, net::SocketAddr, time::SystemTime};
use thiserror::Error;
use tokio::net::{TcpStream, tcp::OwnedWriteHalf};

pub type ServerResult = Result<(), ServerError>;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserId(pub u32);
impl Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}<User>", self.0)
    }
}
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NpcId(pub u32);
impl Display for NpcId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}<Npc>", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub from: Option<ChatTarget>,
    pub to: ChatTarget,
    pub content: String,
    pub timestamp: SystemTime,
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
            timestamp: SystemTime::now(),
            tone: tone.unwrap_or_default(),
        }
    }

    pub fn to_output(&self, is_private: bool) -> String {
        format!(
            "{} {} {} {}: {}\n",
            DateTime::<Local>::from(self.timestamp),
            self.from.unwrap_or_default(),
            self.tone.clone(),
            is_private.then_some("*privately*").unwrap_or(""),
            self.content
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemNotification {
    pub to: UserId,
    pub content: String,
}
impl SystemNotification {
    pub fn to_output(&self) -> String {
        format!(
            "{} System: {}\n",
            DateTime::<Local>::from(SystemTime::now()),
            self.content
        )
    }
}

#[derive(Debug)]
pub enum Event {
    NewClient {
        connection: TcpStream,
        addr: SocketAddr,
    },
    DisconnectClient {
        id: UserId,
    },
    ReceiveUserMessage {
        from: UserId,
        message_raw: String,
    },
    BroadcastMessage {
        message: Message,
    },
    ChangeTarget {
        id: UserId,
        to: ChatTarget,
    },
    NotifyClient {
        notification: SystemNotification,
    },
    Shutdown,
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::NewClient { addr: l_addr, .. }, Self::NewClient { addr: r_addr, .. }) => {
                l_addr == r_addr
            }
            (left, right) => left == right,
        }
    }
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
    pub send_tx: OwnedWriteHalf,
    pub context: ClientContext,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum ChatTarget {
    #[default]
    Global,
    User(UserId),
    Npc(NpcId),
}

impl ChatTarget {
    pub fn user(id: u32) -> Self {
        Self::User(UserId(id))
    }

    pub fn npc(id: u32) -> Self {
        Self::Npc(NpcId(id))
    }
}

impl Display for ChatTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChatTarget::Global => write!(f, "The World"),
            ChatTarget::User(id) => write!(f, "{id}"),
            ChatTarget::Npc(id) => write!(f, "{id}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
// Caches current State of a client
pub struct ClientContext {
    pub current_target: ChatTarget,
    pub tone: MessageTone,
}

/// The emotion that's paired with this message
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessageTone {
    #[default]
    Said,
    Yelled,
    Laughed,
    Whispered,
}

impl Display for MessageTone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            MessageTone::Said => "said",
            MessageTone::Yelled => "yelled",
            MessageTone::Laughed => "laughed",
            MessageTone::Whispered => "whispered",
        };
        write!(f, "{s}")
    }
}
