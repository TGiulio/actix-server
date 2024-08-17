use crate::helpers::spawn_app;

#[tokio::test]
async fn health_check_works() {
    let test_app = spawn_app().await;
    let address = test_app.address;

    // use reqwest to make http requests to our endpoint(s)
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/health_check", address))
        .send()
        .await
        .expect("failed to execute health_check request");

    // verifiy status 200
    assert!(response.status().is_success());
    // verify content length 0
    assert_eq!(response.content_length(), Some(0));
}
