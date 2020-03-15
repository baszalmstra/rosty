use rosty_msg::rosgraph_msgs::Log;
use rosty_msg::rosmsg::RosMsg;
use std::fs::File;
use std::io::Cursor;
use std::process::Command;

#[test]
/// This tests the encoding and decoding of a Log message
fn test_self_encode_decode() {
    // Define some test data
    let name = "Test";
    let msg = "This is a test";
    let topics = vec!["Topic1".to_owned(), "Topic2".to_owned()];

    // Create the log struct
    let mut log = Log::default();
    log.level = 1 as i8;
    log.name = name.to_owned();
    log.msg = msg.to_owned();
    log.topics = topics.clone();

    // Encode it into a buffer
    let mut buffer: Vec<u8> = Vec::new();
    let cursor = Cursor::new(&mut buffer);
    log.encode(cursor).expect("Could not encode message");

    // Decode it back
    let log2 = Log::decode(Cursor::new(&mut buffer)).expect("Could not decode message");

    // See if the data matches
    assert_eq!(log2.level, log.level);
    assert_eq!(log2.name, log.name);
    assert_eq!(log2.msg, log.msg);
    assert_eq!(log2.topics, log.topics);
}

// Uncomment if you want to generate a file
//#[test]
//fn foo() {
//// Define some test data
//let name = "Test";
//let msg = "This is a test";
//let topics = vec!["Topic1".to_owned(), "Topic2".to_owned()];

//// Create the log struct
//let mut log = Log::default();
//log.level = 1 as i8;
//log.name = name.to_owned();
//log.msg = msg.to_owned();
//log.topics = topics.clone();

//let file = File::create("tests/log_rust.bytes").expect("Could not open rust file");
//log.encode(file).expect("Could not encode to file");
//}

#[test]
/// This tests reads a python Log ros message and checks some fields
fn read_python_log() {
    let log_file = File::open("tests/log.bytes").expect("Could not open test file");
    let log = Log::decode(&log_file).expect("Could not decode message");

    assert_eq!(log.name, "Test");
    assert_eq!(log.msg, "This is a test");
    assert_eq!(log.level, 1);
    assert_eq!(log.topics, vec!["Topic1".to_owned(), "Topic2".to_owned()]);
}

#[test]
fn read_rust_log() {
    let mut out = Command::new("python2")
        .arg("tests/read_ros_message.py")
        .spawn()
        .expect("Could not spawn python process");
    assert!(out.wait().expect("Process could not execute succesfully").success())
}
