mod util;
use rosty::Topic;

#[test]
fn subscribe() {
    util::run_with_node(async {
        rosty::subscribe("/rosout", 1,|msg:rosty_msg::rosgraph_msgs::Log| {
            println!("{:?}", msg);
        }).await.unwrap();
    })
}
