mod util;

#[test]
fn register_node() {
    let _roscore = util::run_roscore().unwrap();

    let node = rosty::NodeBuilder::new("register_test");
}