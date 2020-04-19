pub mod util;
use rosty::Topic;

#[test]
fn topics() {
    util::run_with_node(async {
        assert_eq!(
            rosty::topics().await.unwrap(),
            vec![
                Topic {
                    name: "/rosout".to_owned(),
                    data_type: "rosgraph_msgs/Log".to_owned()
                },
                Topic {
                    name: "/rosout_agg".to_owned(),
                    data_type: "rosgraph_msgs/Log".to_owned()
                }
            ]
        );
    })
}
