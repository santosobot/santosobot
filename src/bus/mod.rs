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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_message_bus_creation() {
        let bus = MessageBus::new(10);
        assert_eq!(bus.inbound_size(), 10);
        assert_eq!(bus.outbound_size(), 10);
    }

    #[tokio::test]
    async fn test_message_bus_publish_and_consume_inbound() {
        let mut bus = MessageBus::new(10);
        let test_msg = InboundMessage::new(
            "test".to_string(),
            "user123".to_string(),
            "chat456".to_string(),
            "Test message".to_string(),
        );

        bus.publish_inbound(test_msg.clone()).await;
        let received_msg = bus.consume_inbound().await;

        assert!(received_msg.is_some());
        let received_msg = received_msg.unwrap();
        assert_eq!(received_msg.channel, test_msg.channel);
        assert_eq!(received_msg.content, test_msg.content);
    }

    #[tokio::test]
    async fn test_message_bus_publish_and_consume_outbound() {
        let mut bus = MessageBus::new(10);
        let test_msg = OutboundMessage::new(
            "test".to_string(),
            "chat456".to_string(),
            "Test message".to_string(),
        );

        bus.publish_outbound(test_msg.clone()).await;
        let received_msg = bus.consume_outbound().await;

        assert!(received_msg.is_some());
        let received_msg = received_msg.unwrap();
        assert_eq!(received_msg.channel, test_msg.channel);
        assert_eq!(received_msg.content, test_msg.content);
    }
}
