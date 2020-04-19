use futures::StreamExt;
use std::time::Duration;

pub mod util;

#[test]
fn publish_subscribe() {
    util::run_with_node(async {
        // Start a publisher
        let publisher = rosty::publish::<rosty_msg::std_msgs::String>("/foo", 8)
            .await
            .unwrap();

        // Subscribe to that publisher
        let mut subscriber = rosty::subscribe::<rosty_msg::std_msgs::String>("/foo", 8)
            .await
            .unwrap();

        // Send some data
        tokio::spawn(async move {
            loop {
                let mut msg = rosty_msg::std_msgs::String::default();
                msg.data = "Hello from Rust".to_string();
                publisher.send(msg).await.unwrap();
                tokio::time::delay_for(Duration::from_millis(1)).await;
            }
        });

        let msg = tokio::select!( 
            _ = tokio::time::delay_for(Duration::from_secs(10)) => panic!("topic /foo was never unregistered"),
            msg = subscriber.next() => {msg.unwrap()});
    })
}
