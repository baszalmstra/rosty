use super::master::Master;
use super::slave::Slave;
use crate::node::error::SubscriptionError;
use crate::tcpros::Message;
use std::sync::Arc;

pub struct Subscriber {
    master: Arc<Master>,
    slave: Arc<Slave>,
    name: String,
}

impl Subscriber {
    pub(crate) async fn new<T: Message, F: Fn(T, &str) + Send + 'static>(
        master: Arc<Master>,
        slave: Arc<Slave>,
        name: &str,
        queue_size: usize,
        callback: F,
    ) -> Result<Self, SubscriptionError> {
        // Register the subscription with the slave
        slave
            .add_subscription::<T, F>(name, queue_size, callback)
            .await?;

        // Notify the master that we are subscribing to the given topic. The master will return
        // a list of publishers that publish to the topic we want to subscribe to.
        let publishers = master
            .register_subscriber(name, &T::msg_type())
            .await
            .map_err(|e| SubscriptionError::MasterCommunicationError(e))?;

        // Let the slave know which nodes are publishing data for the topic so that the slave will
        // connect to them to receive the data
        slave
            .add_publishers_to_subscription(name, publishers.into_iter())
            .await?;

        Ok(Self {
            master,
            slave,
            name: name.to_owned(),
        })
    }
}
