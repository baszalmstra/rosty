mod util;

#[test]
fn register_node() {
    let _roscore = util::run_roscore().unwrap();

    rosty::init("node_test").unwrap();
}