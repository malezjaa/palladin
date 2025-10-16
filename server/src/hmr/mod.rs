mod inject;
mod ws;

use serde::Serialize;
use tokio::sync::broadcast;

pub use inject::*;
pub use ws::*;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum HmrMessage {
    #[serde(rename = "update")]
    Update { updates: Vec<Update> },

    #[serde(rename = "full-reload")]
    FullReload,

    #[serde(rename = "connected")]
    Connected,
}

#[derive(Debug, Clone, Serialize)]
pub struct Update {
    pub path: String,
    pub timestamp: u64,
}

pub type HmrBroadcaster = broadcast::Sender<HmrMessage>;

pub fn create_hmr_channel() -> HmrBroadcaster {
    let (tx, _) = broadcast::channel(100);
    tx
}
