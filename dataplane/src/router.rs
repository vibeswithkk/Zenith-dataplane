/// Event routing logic
use crate::Event;
use std::collections::HashMap;
use crossbeam::channel::Sender;

pub struct EventRouter {
    routes: HashMap<u32, Vec<Sender<Event>>>,
}

impl EventRouter {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }
    
    pub fn add_route(&mut self, source_id: u32, sender: Sender<Event>) {
        self.routes.entry(source_id)
            .or_default()
            .push(sender);
    }
    
    pub fn route(&self, event: &Event) {
        if let Some(senders) = self.routes.get(&event.source_id) {
            for sender in senders {
                let _ = sender.try_send(event.clone());
            }
        }
    }
}

impl Default for EventRouter {
    fn default() -> Self {
        Self::new()
    }
}
