mod util;

#[test]
fn test_parameter_api() {
    util::run_with_node(async {
        let parameters = rosty::param_names()
            .await
            .expect("No parameters could be retrieved from the ROS master");
        assert!(
            parameters.len() > 0,
            "No parameters were found on the ROS master"
        );

        let param = rosty::param("test");

        // It should not exist when starting
        assert_eq!(param.exists().await.unwrap(), false);

        // Now we should be able to set a value
        param.set(&"foo".to_owned()).await.unwrap();

        // And the value should be the same
        assert_eq!(param.get::<String>().await.unwrap(), "foo".to_owned());

        // And we should be able to set another value
        param.set(&10i32).await.unwrap();

        // And it should be changed
        assert_eq!(param.get::<i32>().await.unwrap(), 10);

        // Delete this parameter
        param.delete().await.unwrap();

        // And it should no longer exist
        assert_eq!(param.exists().await.unwrap(), false);

        let root_param = rosty::param("/baz");

        // It should not exist when starting
        assert_eq!(root_param.exists().await.unwrap(), false);

        // Set it to a value
        root_param.set(&20i32).await.unwrap();

        // Now search for this param, and check the value
        assert_eq!(root_param.get::<i32>().await.unwrap(), 20);
        //assert_eq!(rosty::search_param::<i32, _>("baz").await.unwrap(), 20);
    });
}
