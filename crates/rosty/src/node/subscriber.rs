use super::master::Master;
use super::slave::Slave;
use crate::node::error::SubscriptionError;
use crate::tcpros::{IncomingMessage, Message};
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

pub struct Subscriber<T: Message> {
    _master: Arc<Master>,
    _slave: Arc<Slave>,
    _name: String,
    channel: mpsc::Receiver<IncomingMessage<T>>,
}

impl<T: Message> Subscriber<T> {
    pub(crate) async fn new(
        master: Arc<Master>,
        slave: Arc<Slave>,
        name: &str,
        queue_size: usize,
    ) -> Result<Self, SubscriptionError> {
        // Register the subscription with the slave
        let channel = slave.add_subscription::<T>(name, queue_size).await?;

        // Notify the master that we are subscribing to the given topic. The master will return
        // a list of publishers that publish to the topic we want to subscribe to.
        let publishers = master
            .register_subscriber(name, &T::msg_type())
            .await
            .map_err(SubscriptionError::MasterCommunicationError)?;

        // Let the slave know which nodes are publishing data for the topic so that the slave will
        // connect to them to receive the data
        slave
            .add_publishers_to_subscription(name, publishers.into_iter())
            .await?;

        Ok(Self {
            _master: master,
            _slave: slave,
            _name: name.to_owned(),
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
