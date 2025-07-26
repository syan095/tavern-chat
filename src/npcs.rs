//! Contains code for NPC info, state and behavior
use std::time::Instant;

#[derive(Debug)]
pub struct Npc {
    name: String,
    state: NpcState,
    last_active: Instant,
}

impl Default for Npc {
    fn default() -> Self {
        Self {
            name: "Unnamed".to_owned(),
            state: Default::default(),
            last_active: Instant::now(),
        }
    }
}

#[derive(Default, Debug)]
pub enum NpcState {
    #[default]
    Idle,
    Disabled,
}
