use super::slave::Slave;
use crate::node::error::SubscriptionError;
use crate::tcpros::{IncomingMessage, Message};
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

pub struct Subscriber<T: Message> {
    slave: Arc<Slave>,
    name: String,
    channel: mpsc::Receiver<IncomingMessage<T>>,
}

impl<T: Message> Subscriber<T> {
    pub(crate) async fn new(
        slave: Arc<Slave>,
        name: &str,
        queue_size: usize,
    ) -> Result<Self, SubscriptionError> {
        // Register the subscription with the slave
        let channel = slave.add_subscription::<T>(name, queue_size).await?;

        Ok(Self {
            slave,
            name: name.to_owned(),
            channel,
        })
    }
}

impl<T: Message> Stream for Subscriber<T> {
    type Item = IncomingMessage<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().channel.poll_recv(cx)
    }
}

impl<T: Message> Drop for Subscriber<T> {
    fn drop(&mut self) {
        let name = self.name.clone();
        let slave = self.slave.clone();
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                slave.remove_subscription(&name).await;
            });
        }
    }
}
