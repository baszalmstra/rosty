#[tokio::main]
async fn main() {
    rosty::init("subscribe_examples").await.unwrap();

    rosty::subscribe("/rosout", 1,|msg:rosty_msg::rosgraph_msgs::Log| {
        println!("{:?}", msg);
    }).await.unwrap();

    rosty::run().await;
}