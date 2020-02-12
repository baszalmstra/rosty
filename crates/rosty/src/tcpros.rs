use std::fmt::Debug;

mod subscriber;

/// Contains information about the definition of a message
struct MessageDefinition {
    /// Full text of message definition (output of `gendeps --cat`)
    message_definition: String,

    /// md5 sum of the message definition
    md5_sum: md5::Digest,

    /// The type name of the message definition
    r#type: String,
}

trait Message: Clone + Debug + Default + PartialEq + Send + 'static {
    /// Returns the message definition associated with a type or None if the message is anonymous.
    fn definition() -> Option<&'static MessageDefinition>;
}