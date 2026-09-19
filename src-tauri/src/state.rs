use crate::db::Db;
use crate::provider::ModelInfo;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// A generation in flight. Cancellation is a flag rather than an aborted task
/// so the stream loop can finish tidily: flush the partial text, mark the row
/// `interrupted`, and tell the UI — instead of vanishing mid-write.
pub struct StreamHandle {
    pub message_id: String,
    pub cancel: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct ModelCache {
    pub fetched_at: Option<Instant>,
    pub models: Vec<ModelInfo>,
}

pub struct AppState {
    pub db: Arc<Db>,
    /// Keyed by conversation id: one live generation per conversation.
    pub streams: Mutex<HashMap<String, StreamHandle>>,
    pub models: Mutex<ModelCache>,
}

impl AppState {
    pub fn new(db: Arc<Db>) -> Self {
        Self {
            db,
            streams: Mutex::new(HashMap::new()),
            models: Mutex::new(ModelCache::default()),
        }
    }

    pub fn is_streaming(&self, conversation_id: &str) -> bool {
        self.streams.lock().unwrap().contains_key(conversation_id)
    }

    pub fn register(&self, conversation_id: &str, handle: StreamHandle) {
        self.streams
            .lock()
            .unwrap()
            .insert(conversation_id.to_string(), handle);
    }

    pub fn unregister(&self, conversation_id: &str) {
        self.streams.lock().unwrap().remove(conversation_id);
    }

    /// Signal one generation to wind down. Returns the message it was writing.
    pub fn cancel(&self, conversation_id: &str) -> Option<String> {
        let map = self.streams.lock().unwrap();
        map.get(conversation_id).map(|h| {
            h.cancel.store(true, std::sync::atomic::Ordering::SeqCst);
            h.message_id.clone()
        })
    }

    /// Used on shutdown so in-flight rows are finalised rather than left
    /// looking live to the next launch.
    pub fn cancel_all(&self) {
        for h in self.streams.lock().unwrap().values() {
            h.cancel.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }
}
