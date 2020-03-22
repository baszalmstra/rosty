use crate::tcpros::{PublisherStream, Message, PublisherError, PublisherSendError};
use crate::node::slave::Slave;
use std::sync::Arc;

pub struct Publisher<T: Message> {
    slave: Arc<Slave>,
    name: String,
    stream: PublisherStream<T>
}

impl<T: Message> Publisher<T> {
    pub(crate) async fn new(
        slave: Arc<Slave>,
        hostname: &str,
        topic: &str,
        queue_size: usize
    ) -> Result<Self, PublisherError> {
        // Register the subscription with the slave
        let stream = slave.add_publication::<T>(hostname, topic, queue_size).await?;

        Ok(Self {
            slave,
            name: topic.to_owned(),
            stream
        })
    }

    pub async fn send(&self, message: &T) -> Result<(), PublisherSendError> {
        self.stream.send(message).await
    }
}

impl<T: Message> Drop for Publisher<T> {
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
