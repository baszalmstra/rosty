use crate::rosxmlrpc::ResponseError;
use crate::tcpros::PublisherConnectError;

#[derive(Fail, Debug)]
pub enum SubscriptionError {
    #[fail(display="a error occured during transport")]
    TransportError(std::io::Error),

    #[fail(display="already subscribed to topic '{}'", topic)]
    DuplicateSubscription { topic: String },

    #[fail(display="communication with the master node failed")]
    MasterCommunicationError(ResponseError),

    #[fail(display="publisher responded with a non-TCPROS protocol: {}", 0)]
    ProtocolMismatch(String),

    #[fail(display="error communicating with publisher")]
    PublisherConnectError(PublisherConnectError),

    #[fail(display="error calling 'requestTopic' on the publisher")]
    RequestTopicError(ResponseError)
}