use std::env;
use std::borrow::Cow;

/// A builder helper class to construct a new ROS `Node`
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NodeArgs {
    name: String,
    master_uri: String,
    hostname: String,
    namespace: String,
}

impl NodeArgs {
    /// Constructs new `NodeArgs` which can be used to construct a new `Node`. By default this
    /// method reads all arguments from the commandline and environment.
    ///
    /// * `default_name` - The default name of the node. This name can be overwritten on the
    ///   command line but must also be specified here as a sensible default.
    pub fn new<S: AsRef<str>>(default_name: S) -> NodeArgs {
        NodeBuilder {
            name: opt_name().unwrap_or_else(|| default_name.as_ref().to_owned()),
            master_uri: master_uri(),
            hostname: hostname(),
            namespace: ensure_starts_with_slash(namespace()),
        }
    }

    /// Explicitly set the name of the node to the specified value.
    pub fn set_name<S: AsRef<str>>(mut self, name: S) -> NodeArgs {
        self.name = name.as_ref().to_owned();
        self
    }

    /// Explicitly set the master URI of the node to the specified value.
    pub fn set_master_uri<S: AsRef<str>>(mut self, uri: S) -> NodeArgs {
        self.master_uri = uri.as_ref().to_owned();
        self
    }

    /// Explicitly set the hostname of the node to the specified value.
    pub fn set_hostname<S: AsRef<str>>(mut self, hostname: S) -> NodeArgs {
        self.hostname = hostname.as_ref().to_owned();
        self
    }

    /// Explicitly set the namespace of the node to the specified value.
    pub fn set_namespace<S: AsRef<str>>(mut self, namespace: S) -> NodeArgs {
        self.namespace = ensure_starts_with_slash(namespace.as_ref().to_owned());
        self
    }
}

/// Ensures that the specified string starts with a `/`
fn ensure_starts_with_slash(namespace: String) -> String {
    if !namespace.starts_with('/') {
        format!("/{}", namespace)
    } else {
        namespace
    }
}

/// Returns a name that was possibly specified on the command line
fn opt_name() -> Option<String> {
    find_arg_with_prefix("__name:=")
}

/// Returns the master URI as taken from the command line, environment variables or default.
fn master_uri() -> String {
    find_arg_with_prefix("__master:=")
        .or_else(|| env::var("ROS_MASTER_URI").ok())
        .unwrap_or_else(|_| "http://localhost:11311".to_owned())
}

/// Returns the hostname of the node. Checks the command line arguments for `__hostname:=`,
/// `__ip:=`, the environment variables `ROS_HOSTNAME` and `ROS_IP` or otherwise uses
/// the system hostname.
fn hostname() -> String {
    find_arg_with_prefix("__hostname:=")
        .or_else(|| find_arg_with_prefix("__ip:="))
        .or_else(|| env::var("ROS_HOSTNAME").ok())
        .or_else(|| env::var("ROS_IP").ok())
        .unwrap_or_else(system_hostname())
}

/// Returns the namespace that hosts the node, checks the command line and ROS_NAMESPACE environment
/// variable.
fn namespace() -> String {
    find_arg_with_prefix("__ns:=")
        .or_else(|| env::var("ROS_NAMESPACE").ok())
        .unwrap_or_default()
}

/// Helper function to find an argument with a given prefix
fn find_arg_with_prefix(prefix: &str) -> Option<String> {
    env::args()
        .skip(1)
        .find(|v| v.starts_with(prefix))
        .map(|v| v.trim_start_matches(prefix).into())
}

/// Returns the system hostname by calling `gethostname`.
///
/// # Panics
///
/// - If the `gethostname` method is unavailable
/// - If the hostname is longer than 256 bytes
/// - If the hostname is not a valid UTF-8 name
fn system_hostname() -> String {
    use nix::unistd::gethostname;
    let mut hostname = [0u8; 256];
    gethostname(&mut hostname)
        .expect("Hostname is either unavailable or too long to fit into buffer");
    let hostname = hostname
        .iter()
        .take_while(|&v| *v != 0u8)
        .cloned()
        .collect::<Vec<_>>();
    String::from_utf8(hostname).expect("Hostname is not legal UTF-8")
}