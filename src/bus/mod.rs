mod events;

pub use events::{InboundMessage, OutboundMessage};

use tokio::sync::mpsc;

#[allow(dead_code)]
pub struct MessageBus {
    inbound: mpsc::Receiver<InboundMessage>,
    outbound: mpsc::Receiver<OutboundMessage>,
    inbound_tx: mpsc::Sender<InboundMessage>,
    outbound_tx: mpsc::Sender<OutboundMessage>,
}

impl MessageBus {
    pub fn new(cap: usize) -> Self {
        let (inbound_tx, inbound) = mpsc::channel(cap);
        let (outbound_tx, outbound) = mpsc::channel(cap);
        
        Self {
            inbound,
            outbound,
            inbound_tx,
            outbound_tx,
        }
    }

    #[allow(dead_code)]
    pub async fn publish_inbound(&self, msg: InboundMessage) {
        let _ = self.inbound_tx.send(msg).await;
    }

    #[allow(dead_code)]
    pub async fn consume_inbound(&mut self) -> Option<InboundMessage> {
        self.inbound.recv().await
    }

    #[allow(dead_code)]
    pub async fn publish_outbound(&self, msg: OutboundMessage) {
        let _ = self.outbound_tx.send(msg).await;
    }

    #[allow(dead_code)]
    pub async fn consume_outbound(&mut self) -> Option<OutboundMessage> {
        self.outbound.recv().await
    }

    #[allow(dead_code)]
    pub fn inbound_size(&self) -> usize {
        self.inbound.capacity()
    }

    #[allow(dead_code)]
    pub fn outbound_size(&self) -> usize {
        self.outbound.capacity()
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new(100)
    }
}
