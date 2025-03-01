use parking_lot::RwLock;
use serde::Serialize;
use std::sync::Arc;
use sysinfo::System;

use crate::{AppState, events::Event};

#[derive(Clone, Serialize)]
pub struct MemoryState {
    used: u64,
    total: u64,
}

pub struct SystemMonitor {
    state: Arc<RwLock<MemoryState>>,
}

impl SystemMonitor {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(MemoryState { used: 0, total: 0 })),
        }
    }

    // Get the current memory state
    pub fn get_state(&self) -> MemoryState {
        self.state.read().clone()
    }

    // Update the memory state
    pub fn update(&self, used: u64, total: u64) {
        let mut state = self.state.write();
        state.used = used;
        state.total = total;
    }
}

// Send regular updates to the event manager and thereby the connected clients
pub async fn send_updates(state: AppState) {
    let mut system = System::new_all();

    loop {
        system.refresh_all();

        // Store
        state
            .monitor
            .update(system.used_memory(), system.total_memory());

        // Send update
        state.channel.send(Event::MemoryState {
            used: system.used_memory(),
            total: system.total_memory(),
        });

        // Sleep before next update
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}
