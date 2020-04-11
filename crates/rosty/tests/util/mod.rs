use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;

use std::collections::HashSet;
use std::future::Future;

use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time;
use std::time::Duration;
use std::{env, io};

/// Enable RAII usage of a ros process. When the process is dropped it's send the SIGINT signal to
/// properly shut it down.
pub struct ROSChildProcess(Child);

impl ROSChildProcess {
    pub fn spawn(command: &mut Command) -> io::Result<ROSChildProcess> {
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command.spawn().map(ROSChildProcess)
    }
}

impl Drop for ROSChildProcess {
    fn drop(&mut self) {
        let pid = Pid::from_raw(self.0.id() as i32);
        kill(pid, Signal::SIGINT).unwrap();
        self.0.wait().unwrap();
    }
}

/// Start the ROS core on a specific port. This also sets the ROS_MASTER_URI environment variable to
/// the correct value.
pub fn run_roscore() -> io::Result<ROSChildProcess> {
    let port = 11400;
    println!("Starting roscore on port: {}", port);
    env::set_var("ROS_MASTER_URI", format!("http://localhost:{}", port));
    let roscore =
        ROSChildProcess::spawn(&mut Command::new("roscore").arg("-p").arg(format!("{}", port)))?;
    print!("Waiting for roscore to come online...");
    await_roscore()?;
    println!("\tdone!");
    Ok(roscore)
}

/// Set the `/use_sim_time` parameter on the parameter server
fn set_use_sim_time() -> io::Result<Child> {
    Command::new("rosparam")
        .args(&["set", "/use_sim_time", "true"])
        .spawn()
}

/// Helper function to check if the roscore is online.
fn rostopic_listing_succeeds() -> bool {
    let result = Command::new("rostopic").arg("list").output().unwrap();
    if !result.status.success() {
        return false;
    }

    let output = String::from_utf8(result.stdout);
    if let Ok(result) = output {
        let topics = result
            .split_whitespace()
            .map(ToOwned::to_owned)
            .collect::<HashSet<String>>();
        topics.contains("/rosout") && topics.contains("/rosout_agg")
    } else {
        false
    }
}

/// Publish a message on the clock topic
pub fn publish_clock() -> io::Result<Child> {
    Command::new("rostopic")
        .args(&["pub", "/clock", "rosgraph_msgs/Clock", "[100, 1000]"])
        .spawn()
}

/// Waits until the roscore comes online by polling.
fn await_roscore() -> io::Result<()> {
    while !rostopic_listing_succeeds() {
        sleep(Duration::from_millis(100));
    }
    Ok(())
}

pub fn run_with_node(generator: impl Future<Output = ()>) {
    tokio_test::block_on(async move {
        let _roscore = run_roscore().unwrap();
        rosty::init("test").await.unwrap();
        generator.await;
    })
}

/// Same as `run_with_node` but also runs with `/use_sim_time` set
pub fn run_with_node_simtime(generator: impl Future<Output = ()>) {
    tokio_test::block_on(async move {
        let _roscore = run_roscore().unwrap();
        // Use the sim time
        set_use_sim_time().expect("Cannot set the simtime parameter");
        // Wait for the param to actually be set
        tokio::time::delay_for(time::Duration::from_millis(1000)).await;
        rosty::init("test").await.unwrap();
        generator.await;
    })
}
