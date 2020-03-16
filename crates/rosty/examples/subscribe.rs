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
    rosty::subscribe("/rosout", 1, |msg: rosty_msg::rosgraph_msgs::Log| {
        println!("{:?}", msg);
    })
    .await
    .unwrap();

    // Run the node until it quits
    rosty::run().await;
}
