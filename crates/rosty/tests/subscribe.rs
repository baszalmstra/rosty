mod util;
use rosty::Topic;
use tokio::process::Command;

#[test]
fn subscribe() {
    util::run_with_node(async {
        let test_string = "hello, world!";

        rosty::subscribe("/test_subscriber", 1, |msg: rosty_msg::std_msgs::String| {
            panic!("hello drol")
        })
        .await
        .unwrap();

        Command::new("rostopic")
            .arg("pub")
            .arg("/test_subscriber")
            .arg("std_msgs/String")
            .arg(test_string)
            .spawn()
            .unwrap()
            .await
            .unwrap();
    })
}
