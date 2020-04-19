use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), failure::Error> {
    // Setup logging to the console
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Initialize a rosty node
    rosty::init("publish_example").await?;

    let publisher = rosty::publish::<rosty_msg::std_msgs::String>("/foo", 8).await?;

    let mut counter = 0;
    while !rosty::is_awaiting_shutdown() {
        let mut msg = rosty_msg::std_msgs::String::default();
        msg.data = format!("Hello, timmie! {}", counter);
        counter += 1;
        publisher.send(msg).await.unwrap();
        tokio::time::delay_for(Duration::from_millis(1)).await;
    }

    // Run the node until it quits
    rosty::run().await;

    Ok(())
}
