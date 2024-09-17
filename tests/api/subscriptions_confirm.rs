use crate::helpers::spawn_app;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_400() {
    // prepare what's needed
    let test_app = spawn_app().await;

    let response = reqwest::get(&format!("{}/subscriptions/confirm", test_app.address))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn confirmations_with_invalid_token_are_rejected_with_400() {
    // prepare what's needed
    let test_app = spawn_app().await;

    let response = reqwest::get(&format!(
        "{}/subscriptions/confirm?subscription_token=KYu7R2TPDCAy1rT141uOExlVVf", //invalid token
        test_app.address
    ))
    .await
    .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}
#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    // prepare what's needed
    let test_app = spawn_app().await;
    let body = "name=Alpha%20Centauri&email=alphacentauri%40smail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body.into()).await;
    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = test_app.get_confirmation_links(&email_request);

    let response = reqwest::get(confirmation_links.html).await.unwrap();

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn visiting_confirmation_link_confirms_a_subscriber() {
    // prepare what's needed
    let test_app = spawn_app().await;
    let body = "name=Alpha%20Centauri&email=alphacentauri%40smail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body.into()).await;
    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = test_app.get_confirmation_links(&email_request);

    // visit the link
    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to retrieve saved subscription");

    assert_eq!(saved.email, "alphacentauri@smail.com");
    assert_eq!(saved.name, "Alpha Centauri");
    assert_eq!(saved.status, "confirmed");
}

#[tokio::test]
async fn subscription_confirmation_fails_if_there_is_a_fatal_database_error() {
    let test_app = spawn_app().await;
    let body = "name=Alpha%20Centauri&email=alphacentauri%40smail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body.into()).await;

    // sabotage the database
    sqlx::query!("DROP TABLE subscription_tokens;",)
        .execute(&test_app.db_pool)
        .await
        .unwrap();

    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = test_app.get_confirmation_links(&email_request);

    // visit the link
    let response = reqwest::get(confirmation_links.html).await.unwrap();

    assert_eq!(response.status().as_u16(), 500);
}

#[tokio::test]
async fn subscription_confirmation_fails_with_401_if_token_has_no_subscriber() {
    let test_app = spawn_app().await;
    let body = "name=Alpha%20Centauri&email=alphacentauri%40smail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body.into()).await;

    // sabotage the database
    sqlx::query!("DELETE FROM subscription_tokens;",)
        .execute(&test_app.db_pool)
        .await
        .unwrap();

    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = test_app.get_confirmation_links(&email_request);

    // visit the link
    let response = reqwest::get(confirmation_links.html).await.unwrap();

    assert_eq!(response.status().as_u16(), 401);
}
