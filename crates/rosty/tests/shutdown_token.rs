mod util;
use futures::{future::maybe_done, pin_mut};
use rosty::Topic;

#[test]
fn shutdown_token() {
    util::run_with_node(async {
        // Get the future to run the node and wrap it so we can check its output
        let run_future = maybe_done(rosty::run());

        // Pin the future to the stack
        pin_mut!(run_future);

        // The node should not be done yet
        assert_eq!(run_future.as_mut().output_mut(), None);

        tokio::task::yield_now().await;
        assert_eq!(run_future.as_mut().output_mut(), None);

        // Now trigger the node to shut down
        rosty::shutdown();

        // Wait for the future to finish (which should be timely)
        run_future.as_mut().await;
    });
}

