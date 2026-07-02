use crate::ports::event_bus::{EventBus, StreamEvent};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

const CHANNEL_CAPACITY: usize = 100;

pub struct MemoryEventBus {
    channels: Arc<Mutex<HashMap<String, broadcast::Sender<StreamEvent>>>>,
}

impl MemoryEventBus {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl EventBus for MemoryEventBus {
    fn emit(&self, thread_id: &str, event: StreamEvent) {
        let mut channels = self.channels.lock().unwrap();
        let sender = channels
            .entry(thread_id.to_string())
            .or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0);
        let _ = sender.send(event);
    }

    fn subscribe(&self, thread_id: &str) -> broadcast::Receiver<StreamEvent> {
        let mut channels = self.channels.lock().unwrap();
        let sender = channels
            .entry(thread_id.to_string())
            .or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0);
        sender.subscribe()
    }
}
