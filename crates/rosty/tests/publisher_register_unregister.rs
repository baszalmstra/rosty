use std::time::Duration;

pub mod util;

#[test]
fn publisher_register_unregister() {
    util::run_with_node(async {
        let has_topic_foo = || async { util::list_topics().unwrap().iter().any(|t| t == "/foo") };

        let wait_for_topic_foo = |is_available: bool| async move {
            loop {
                if has_topic_foo().await == is_available {
                    return;
                }
                tokio::time::delay_for(Duration::from_millis(100)).await;
            }
        };

        // Initially the topic should not be available
        assert_eq!(has_topic_foo().await, false);
        println!("✓ /foo is initially not available.");

        let publisher = rosty::publish::<rosty_msg::std_msgs::String>("/foo", 8)
            .await
            .unwrap();

        // Now there should be a topic registered on the master
        tokio::select!(
            _ = tokio::time::delay_for(Duration::from_secs(10)) => panic!("topic /foo was never registered"),
            _ = wait_for_topic_foo(true) => {});

        println!("✓ /foo is now available.");

        // Drop the publisher
        drop(publisher);

        // Now the topic should go away
        tokio::select!(
            _ = tokio::time::delay_for(Duration::from_secs(30)) => panic!("topic /foo was never unregistered"),
            _ = wait_for_topic_foo(false) => {});

        println!("✓ /foo is no longer available.");
    });
}
