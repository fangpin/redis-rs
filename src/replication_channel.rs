use std::sync::OnceLock;

use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex, MutexGuard,
};

use crate::protocol::Protocol;

static TX: OnceLock<Sender<(Protocol, u64)>> = OnceLock::new();
static RX: OnceLock<Mutex<Receiver<(Protocol, u64)>>> = OnceLock::new();

pub fn init() {
    let (tx, rx) = mpsc::channel(4096);
    TX.set(tx).expect("Sender should only be initialized once");
    RX.set(Mutex::new(rx))
        .expect("Receiver should only be initialized once");
}

pub fn sender() -> Option<Sender<(Protocol, u64)>> {
    TX.get().map(|tx| tx.clone())
}

pub async fn receiver() -> Option<MutexGuard<'static, Receiver<(Protocol, u64)>>> {
    if let Some(rx_mutex) = RX.get() {
        Some(rx_mutex.lock().await)
    } else {
        None
    }
}
