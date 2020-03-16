use futures::StreamExt;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    // Setup logging to the console
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Initialize a rosty node
    rosty::init("subscribe_examples").await.unwrap();

    // Subscribe to a topic
    tokio::spawn(rosty::subscribe::<rosty_msg::rosgraph_msgs::Log>("/rosout", 1)
        .await
        .unwrap()
        .for_each(|(_, message)| async move {
            println!("{:?}", message);
        }));

    // Run the node until it quits
    rosty::run().await;
}
