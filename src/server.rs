//! Contains the Server struct for the tavern.
//! Stores all essential information in this centralized, global instance.

use futures::future::join_all;
use std::collections::{HashMap, VecDeque};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        TcpListener,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    sync::{mpsc, watch},
    task::JoinHandle,
};

use crate::common::*;
use crate::npcs::Npc;

pub const MESSAGE_HISTORY_LEN: usize = 100usize;
pub const TCP_PORT: &str = "127.0.0.1:8080";

#[derive(Debug)]
pub struct TavernServer {
    message_log: VecDeque<Message>,
    npcs: Vec<Npc>,
    clients: HashMap<UserId, Client>,
    next_user_id: UserId,
    event_tx: mpsc::Sender<Event>,
    event_rx: mpsc::Receiver<Event>,
}

impl TavernServer {
    pub fn new() -> (Self, mpsc::Sender<Event>) {
        let (event_tx, event_rx) = mpsc::channel::<Event>(100);
        (
            TavernServer {
                message_log: Default::default(),
                npcs: Default::default(),
                clients: Default::default(),
                next_user_id: Default::default(),
                event_tx: event_tx.clone(),
                event_rx,
            },
            event_tx,
        )
    }

    /// Runs the main loop
    pub async fn run(&mut self) -> anyhow::Result<()> {
        println!("☀️ Starting Tavern Chat server! Welcome!");

        // Create event channel
        let (shutdown_tx, shutdown_rx) = watch::channel(());

        // Initiate TCP connection loop
        let mut client_handles =
            vec![manage_tcp_connections(self.event_tx.clone(), shutdown_rx.clone()).await?];

        while let Some(event) = self.event_rx.recv().await {
            println!("New event: {:?}", event);
            match event {
                Event::NewClient { connection } => {
                    // Assign a new ID to a new client.
                    let id = self.next_user_id;
                    self.next_user_id += 1;

                    let (read_half, write_half) = connection.into_split();
                    self.clients.insert(
                        id,
                        Client {
                            id,
                            send_tx: write_half,
                            context: Default::default(),
                        },
                    );
                    client_handles.push(watch_client(
                        id,
                        read_half,
                        self.event_tx.clone(),
                        shutdown_rx.clone(),
                    ));
                }
                Event::DisconnectClient { id } => self.remove_clients(id),
                Event::ReceiveMessage { from, message_raw } => {
                    let _ = crate::parser::parse_incoming_message(
                        from,
                        message_raw,
                        self.event_tx.clone(),
                    )
                    .await;
                }
                Event::BroadcastMessage { message } => self.broadcast_message(message).await,
                Event::Shutdown => {
                    // Shutdown all spawned threads.
                    let _ = shutdown_tx.send(());
                    self.shutdown();
                    break;
                }
            }
        }

        // Wait for all threads to shutdown
        let _ = join_all(client_handles);

        println!("🌙 Tavern Chat server shutdown! So long!");
        Ok(())
    }

    /// Trigger server shutdown. Teardown everything cleanly.
    pub fn shutdown(&mut self) {
        // Close all existing Client's Tcp connection.
        // Dropping the write half closes to the connection.
        self.clients.clear();
    }

    /// Close a Client's Tcp connection.
    pub fn remove_clients(&mut self, id: UserId) {
        // Dropping the write half closes the connection.
        self.clients.remove(&id);
    }

    /// Broadcast a new message to listeners of the server.
    pub async fn broadcast_message(&mut self, message: Message) {
        // Insert the new message into the log.
        self.message_log.push_back(message.clone());
        if self.message_log.len() > MESSAGE_HISTORY_LEN {
            let _ = self.message_log.pop_front();
        }

        let to_client =
            async |send_tx: &mut OwnedWriteHalf, id: UserId, message: String| -> ServerResult {
                // Ignore error when broadcasting.
                send_tx
                    .write_all(message.as_bytes())
                    .await
                    .map_err(|_| ServerError::TcpConnectionFailed(id))?;
                send_tx
                    .flush()
                    .await
                    .map_err(|_| ServerError::TcpConnectionFailed(id))?;
                Ok(())
            };
        let mut failed_client = vec![];

        if let Err(e) = match message.to {
            ChatTarget::Global => {
                // Broadcast the message to all clients
                println!("Global: {:?}", message.content.clone());
                for (id, client) in self.clients.iter_mut() {
                    if let Err(_) = to_client(&mut client.send_tx, *id, message.to_output()).await {
                        failed_client.push(*id);
                    }
                }
                Ok(())
            }
            ChatTarget::Client(id) => {
                if let Some(client) = self.clients.get_mut(&id) {
                    to_client(&mut client.send_tx, id, message.content.clone())
                        .await
                        .inspect_err(|_| {
                            failed_client.push(id);
                        })
                } else {
                    Err(ServerError::InvalidMessageTarget(message.to))
                }
            }
            ChatTarget::Npc(_id) => todo!("NPC behavior to be implemented later"),
        } {
            // Send reply to Client user.
            if let Some(ChatTarget::Client(sender)) = message.from {
                let _ = self
                    .event_tx
                    .send(Event::BroadcastMessage {
                        message: Message::new(
                            None,
                            ChatTarget::Client(sender),
                            format!("Failed to send message: {:?}", e).as_str(),
                            None,
                        ),
                    })
                    .await;
            }
        }

        // Remove bad connections
        for id in failed_client.into_iter() {
            let _ = self.event_tx.send(Event::DisconnectClient { id }).await;
        }
    }
}

async fn manage_tcp_connections(
    event_dispatch: mpsc::Sender<Event>,
    mut shutdown: watch::Receiver<()>,
) -> anyhow::Result<JoinHandle<()>> {
    let listener = TcpListener::bind(TCP_PORT).await?;

    Ok(tokio::spawn(async move {
        println!(
            "☎️ Rust Tavern server awaiting connections on {:?}",
            TCP_PORT
        );

        loop {
            tokio::select! {
                Ok((socket, addr)) = listener.accept() => {
                    println!("🍺 New client connected: {addr}");
                    let _ = event_dispatch.send(Event::NewClient { connection: socket }).await;
                }
                Ok(()) = shutdown.changed() => {
                    break;
                }
            }
        }
    }))
}

/// A new TCP client has been connected to the server.
fn watch_client(
    id: UserId,
    read_half: OwnedReadHalf,
    event_tx: mpsc::Sender<Event>,
    mut shutdown_rx: watch::Receiver<()>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut lines = BufReader::new(read_half).lines();
        loop {
            tokio::select! {
                res  = lines.next_line() => {
                    match res {
                        Ok(Some(incoming)) => {
                            let _ = event_tx.send(Event::ReceiveMessage{from: ChatTarget::Client(id), message_raw: incoming}).await;
                        },
                        Ok(None) | Err(_) => {
                            println!("❌ Error in connecting to user: {:?}", id);
                            let _ = event_tx.send(Event::DisconnectClient { id }).await;
                            break;
                        },
                    }
                }
                Ok(()) = shutdown_rx.changed() => {
                    break;
                }
            }
        }
    })
}
