mod util;
use futures::StreamExt;
use tokio::process::Command;

#[test]
fn subscribe() {
    util::run_with_node(async {
        let test_string = "hello, world!";

        tokio::spawn(
            rosty::subscribe::<rosty_msg::std_msgs::String>("/test_subscriber", 1)
                .await
                .unwrap()
                .for_each(move |(_, message)| async move {
                    if &message.data == test_string {
                        rosty::shutdown()
                    }
                }),
        );

        tokio::spawn(
            Command::new("rostopic")
                .arg("pub")
                .arg("/test_subscriber")
                .arg("std_msgs/String")
                .arg(test_string)
                .spawn()
                .unwrap(),
        );

        rosty::run().await
    })
}
