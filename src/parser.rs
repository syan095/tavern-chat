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
        let (command, msg) = (*split.get(0).unwrap(), *split.get(1).unwrap_or(&""));

        match command.to_ascii_lowercase().as_str() {
            // Command related to Saying something
            "/say" | "/s" => {
                reply =
                    say_something(from, msg, client_ctx, &event_tx, Some(MessageTone::Said)).await
            }
            "/yell" => {
                reply =
                    say_something(from, msg, client_ctx, &event_tx, Some(MessageTone::Yelled)).await
            }
            "/laugh" => {
                reply = say_something(from, msg, client_ctx, &event_tx, Some(MessageTone::Laughed))
                    .await
            }
            "/whisper" | "/w" => {
                reply = say_something(
                    from,
                    msg,
                    client_ctx,
                    &event_tx,
                    Some(MessageTone::Whispered),
                )
                .await
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
            // Emote
            "/wave" => {
                say_something(
                    from,
                    format!("You waved at {}. Wassup?", client_ctx.current_target).as_str(),
                    client_ctx,
                    &event_tx,
                    None,
                )
                .await;
            }
            "/poke" => {
                say_something(
                    from,
                    format!("You poked {}. Hey!", client_ctx.current_target).as_str(),
                    client_ctx,
                    &event_tx,
                    None,
                )
                .await;
            }
            "/lol" => {
                say_something(
                    from,
                    "You laughed out loud. A ha HA!",
                    client_ctx,
                    &event_tx,
                    Some(MessageTone::Laughed),
                )
                .await;
            }
            "/cry" => {
                say_something(
                    from,
                    format!(
                        "You cried on {}'s shoulder. There there.",
                        client_ctx.current_target
                    )
                    .as_str(),
                    client_ctx,
                    &event_tx,
                    None,
                )
                .await;
            }
            "/dance" => {
                say_something(
                    from,
                    "You danced on top of a table! What a jolly time!",
                    client_ctx,
                    &event_tx,
                    None,
                )
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
    } else {
        // Say the message to the current target
        reply = say_something(from, message_raw.as_str(), client_ctx, &event_tx, None).await
    }

    if let Some(reply) = reply {
        let _ = event_tx
            .send(Event::BroadcastMessage {
                message: Message::new(
                    None,
                    ChatTarget::User(from),
                    format!("{}\n{} >", reply, client_ctx.tone).as_str(),
                    Default::default(),
                ),
            })
            .await;
    }

    Ok(())
}

async fn say_something(
    from: UserId,
    msg: &str,
    client_ctx: &mut ClientContext,
    event_tx: &Sender<Event>,
    new_tone: Option<MessageTone>,
) -> Option<String> {
    let tone = if let Some(tone) = new_tone {
        client_ctx.tone = tone;
        tone
    } else {
        client_ctx.tone
    };

    // Send message if content is non-empty
    if !msg.is_empty() {
        let from = ChatTarget::User(from);
        let to = client_ctx.current_target;

        // Send message out
        let _ = event_tx
            .send(Event::BroadcastMessage {
                message: Message::new(Some(from), to, msg, Some(tone)),
            })
            .await;
    }

    // Reply back if the message is not a Global broadcast.
    match client_ctx.current_target {
        ChatTarget::User(id) => Some(format!("To {}: {}", id, msg)),
        ChatTarget::Npc(id) => Some(format!("To {}: {}", id, msg)),
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
