use crate::node::clock::Clock;
use crate::node::slave::Slave;
use crate::tcpros::{Message, PublisherError, PublisherSendError, PublisherStream};
use failure::_core::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct Publisher<T: Message> {
    _info: Arc<PublisherInfo>,
    stream: PublisherStream<T>,
    clock: Arc<Clock>,
    seq: Arc<AtomicUsize>,
}

impl<T: Message> Publisher<T> {
    pub(crate) async fn new(
        slave: Arc<Slave>,
        hostname: &str,
        topic: &str,
        queue_size: usize,
        clock: Arc<Clock>,
    ) -> Result<Self, PublisherError> {
        // Register the subscription with the slave
        let stream = slave
            .add_publication::<T>(hostname, topic, queue_size)
            .await?;

        Ok(Self {
            _info: Arc::new(PublisherInfo {
                name: topic.to_owned(),
                slave,
            }),
            clock,
            stream,
            seq: Arc::new(AtomicUsize::new(0)),
        })
    }

    pub async fn send(&self, mut message: T) -> Result<(), PublisherSendError> {
        if let Some(header) = message.header_mut() {
            header.stamp = self.clock.now().expect("missing clock time");
            header.seq = self.seq.fetch_add(1, Ordering::AcqRel) as u32
        }
        self.stream.send(message).await
    }
}

struct PublisherInfo {
    name: String,
    slave: Arc<Slave>,
}

impl Drop for PublisherInfo {
    fn drop(&mut self) {
        let name = self.name.clone();
        let slave = self.slave.clone();
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                slave.remove_publisher(&name).await;
            });
        }
    }
}
