use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyEvent};

pub struct EventStream {
    _phantom: std::marker::PhantomData<()>,
}

impl EventStream {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    pub async fn next(&mut self) -> Result<Option<Event>> {
        // Poll for events with a timeout to allow for async updates
        if event::poll(Duration::from_millis(100))? {
            Ok(Some(event::read()?))
        } else {
            Ok(None)
        }
    }
}

impl Default for EventStream {
    fn default() -> Self {
        Self::new()
    }
}

pub fn extract_key_event(event: Event) -> Option<KeyEvent> {
    match event {
        Event::Key(key) => Some(key),
        _ => None,
    }
}
