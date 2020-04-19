pub mod util;

#[test]
fn no_sim_time() {
    util::run_with_node(async {
        // Don't use simtime normally
        assert!(!rosty::is_using_sim_time());

        // Check if we are getting a time from the now function
        assert!(rosty::now().seconds() > 0.0,)
    });
}
