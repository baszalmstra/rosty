use rosty_xmlrpc::{Client, Fault, ServerBuilder};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio_test::block_on;

#[macro_use]
extern crate serde_derive;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct TestStruct {
    pub foo: i32,
    pub bar: String,
}

async fn echo(v: TestStruct) -> Result<TestStruct, Fault> {
    Ok(v)
}

async fn double(mut v: TestStruct) -> Result<TestStruct, Fault> {
    v.foo *= 2;
    v.bar = format!("{0}{0}", v.bar);
    Ok(v)
}

#[test]
fn client_server() {
    block_on(async {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7654);
        let mut server = ServerBuilder::new();

        server.register_simple_async("echo", echo);
        server.register_simple_async("double", double);

        let server_shutdown_signal = futures::future::pending();
        tokio::spawn(server.bind(&addr, server_shutdown_signal).unwrap());

        let mut client = Client::new();
        let req = TestStruct {
            foo: 42,
            bar: "baz".to_owned(),
        };

        println!("Sending: {:?}", req);
        let uri = "http://localhost:7654/".parse().unwrap();
        let res: Result<TestStruct, Fault> = client.call(&uri, "echo", req.clone()).await.unwrap();
        assert_eq!(res, Ok(req.clone()));

        let res: Result<TestStruct, Fault> =
            client.call(&uri, "double", req.clone()).await.unwrap();
        assert_eq!(
            res,
            Ok(TestStruct {
                foo: 84,
                bar: "bazbaz".to_owned()
            })
        );

        let res: Result<TestStruct, Fault> =
            client.call(&uri, "invalid", req.clone()).await.unwrap();
        assert!(res.is_err());
    })
}
