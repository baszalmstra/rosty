pub mod util;
use tokio::time;

#[test]
fn use_sim_time() {
    util::run_with_node_simtime(async {
        // Check if the parameter is set and equals true
        let param = rosty::param("/use_sim_time");
        assert!(
            param.exists().await.unwrap(),
            "/use_sim_time does not exist"
        );
        assert!(
            param.get::<bool>().await.unwrap(),
            "/use_sim_time is not true"
        );

        // In that case is_using_sim_time should also be true
        assert!(rosty::is_using_sim_time());

        let mut child = util::publish_clock().unwrap();
        // Wait for the message to be published
        tokio::time::delay_for(time::Duration::from_millis(500)).await;

        // Check if we get the value published
        let duration = rosty::now();
        assert_eq!(duration.sec, 100);
        assert_eq!(duration.nsec, 1000);
        child.kill().expect("Could not stop clock publisher");
    })
}
