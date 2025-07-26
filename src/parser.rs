//! Contains logic that parses a string into proper event
//! We only need to parse incoming messages from a user.

use crate::common::*;
use tokio::sync::mpsc::Sender;

pub async fn parse_incoming_message(
    from: UserId,
    message_raw: String,
    event_tx: Sender<Event>,
    client_ctx: &mut ClientContext,
) -> ServerResult {
    println!("{:?}: {:?}", from, message_raw);
    if message_raw.is_empty() {
        return Ok(());
    }

    let mut reply = None;

    if message_raw.starts_with('/') {
        let split = message_raw.splitn(2, " ").into_iter().collect::<Vec<_>>();
        let (command, msg) = (split[0], split[1]);

        match command.to_ascii_lowercase().as_str() {
            // Command related to Saying something
            "/say" => {
                reply = say_with_tone(from, MessageTone::Said, msg, client_ctx, &event_tx).await
            }
            "/yell" => {
                reply = say_with_tone(from, MessageTone::Yelled, msg, client_ctx, &event_tx).await
            }
            "/laughed" => {
                reply = say_with_tone(from, MessageTone::Laughed, msg, client_ctx, &event_tx).await
            }
            "/whispered" => {
                reply =
                    say_with_tone(from, MessageTone::Whispered, msg, client_ctx, &event_tx).await
            }
            // Set chat target
            "/to_user" => {
                if let Ok(target_id) = msg.parse::<u32>() {
                    let _ = event_tx
                        .send(Event::ChangeTarget {
                            id: from,
                            to: ChatTarget::user(target_id),
                        })
                        .await;
                } else {
                    reply = Some("Invalid target. please use /to_user <id>".to_string());
                }
            }
            // Change chat target
            "/to_npc" => {
                if let Ok(target_id) = msg.parse::<u32>() {
                    let _ = event_tx
                        .send(Event::ChangeTarget {
                            id: from,
                            to: ChatTarget::npc(target_id),
                        })
                        .await;
                } else {
                    reply = Some("Invalid target. please use /to_npc <id>".to_string());
                }
            }
            "/to_world" | "/to_everyone" | "/global" => {
                let _ = event_tx
                    .send(Event::ChangeTarget {
                        id: from,
                        to: ChatTarget::Global,
                    })
                    .await;
            }

            // System commands
            "/shutdown" => {
                let _ = event_tx.send(Event::Shutdown).await;
            }
            _ => {
                // Unknown command.
                reply = Some("Unknown command.".to_string());
            }
        }
    }

    if let Some(reply) = reply {
        let _ = event_tx
            .send(Event::BroadcastMessage {
                message: Message::new(
                    None,
                    ChatTarget::User(from),
                    reply.as_str(),
                    Default::default(),
                ),
            })
            .await;
    }

    Ok(())
}

async fn say_with_tone(
    from: UserId,
    tone: MessageTone,
    msg: &str,
    client_ctx: &mut ClientContext,
    event_tx: &Sender<Event>,
) -> Option<String> {
    client_ctx.tone = tone;

    let from = ChatTarget::User(from);
    let to = client_ctx.current_target;

    // Send message out
    let _ = event_tx
        .send(Event::BroadcastMessage {
            message: Message::new(Some(from), to, msg, Some(tone)),
        })
        .await;

    // Reply back if the message is not a Global broadcast.
    match client_ctx.current_target {
        ChatTarget::User(id) => Some(format!("To {}: {} \nSay > ", id, msg)),
        ChatTarget::Npc(id) => Some(format!("To {}: {} \nSay > ", id, msg)),
        _ => None,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::{Duration, SystemTime};
    use tokio::{sync::mpsc, time::timeout};

    const SENDER: UserId = UserId(3u32);

    async fn assert_parse_event(input_and_event: Vec<(&str, Event, &mut ClientContext)>) {
        let (tx, mut rx) = mpsc::channel::<Event>(100);

        for (input, expected_event, ctx) in input_and_event.into_iter() {
            assert!(
                parse_incoming_message(SENDER, input.to_string(), tx.clone(), ctx)
                    .await
                    .is_ok()
            );
            let actual_event = match timeout(Duration::from_secs(1), rx.recv()).await {
                Ok(Some(event)) => event,
                Ok(None) | Err(_) => {
                    assert!(
                        false,
                        "Failed to receive Event from the channel. \ninput: {:?}, expected_event: {:?}",
                        input, expected_event
                    );
                    return;
                }
            };

            assert_eq!(expected_event, actual_event);
        }
    }

    #[tokio::test]
    async fn can_parse_say_commands_with_default_ctx() {
        let mut ctx = ClientContext::default();
        assert_parse_event(vec![(
            "/yell hello world!",
            Event::BroadcastMessage {
                message: Message {
                    from: Some(ChatTarget::User(SENDER)),
                    to: ChatTarget::Global,
                    content: "hello world!".to_string(),
                    timestamp: SystemTime::now(),
                    tone: MessageTone::Yelled,
                },
            },
            &mut ctx,
        )])
        .await;
    }
}
