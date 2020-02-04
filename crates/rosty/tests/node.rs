mod util;

#[tokio::test]
async fn register_node() {
    let _roscore = util::run_roscore().unwrap();

    // Construct the ROS node and bind it to the local address
    rosty::init("node_test").await.unwrap();
}
