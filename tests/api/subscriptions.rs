use crate::helpers::spawn_app;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    // prepare what's needed
    let test_app = spawn_app().await;
    let body = "name=Alpha%20Centauri&email=alphacentauri%40smail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    // make the request
    let response = test_app.post_subscriptions(body.into()).await;

    // verify
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    // prepare what's needed
    let test_app = spawn_app().await;
    let db_pool = &test_app.db_pool;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    // make the request
    let body = "name=Alpha%20Centauri&email=alphacentauri%40smail.com";
    let _response = test_app.post_subscriptions(body.into()).await;

    let saved = sqlx::query!("SELECT email, name, status FROM public.subscriptions")
        .fetch_one(db_pool)
        .await
        .expect("cannot retrieve subscriber");

    assert_eq!(saved.email, "alphacentauri@smail.com");
    assert_eq!(saved.name, "Alpha Centauri");
    assert_eq!(saved.status, "pending_confirmation");
}

#[tokio::test]
async fn subscribe_sends_confirmation_email_for_valid_data() {
    let test_app = spawn_app().await;
    let body = "name=Alpha%20Centauri&email=alphacentauri%40smail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body.into()).await;
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
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

    // the two links should be identical
    assert_eq!(confirmation_links.html, confirmation_links.plain_text);
}

#[tokio::test]
async fn subscribe_returns_400_for_missing_form_data() {
    // prepare what's needed
    let test_app = spawn_app().await;
    let test_cases = vec![
        ("name=Alpha%20Centauri", "missing email"),
        ("email=alphacentauri%40smail.com", "missing name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = test_app.post_subscriptions(invalid_body.into()).await;

        assert_eq!(
            response.status().as_u16(),
            400,
            "Tha API did not fail with 400 code when the body was {}",
            error_message
        );
    }
}
#[tokio::test]
async fn subscribe_returns_400_when_fields_are_present_but_invalid() {
    // prepare what's needed
    let test_app = spawn_app().await;
    let test_cases = vec![
        ("name=&email=alphacentauri%40smail.com", "empty name"),
        ("name=Alpha%20Centauri&email", "empty email"),
        (
            "name=Alpha%20Centauri&email=certainly-not-an-email",
            "invalid email",
        ),
    ];

    for (body, error_message) in test_cases {
        let response = test_app.post_subscriptions(body.into()).await;

        assert_eq!(
            response.status().as_u16(),
            400,
            "The API did not fail with 400 code when the body was {}",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_resends_confirmation_email_for_duplicate_email() {
    let test_app = spawn_app().await;
    let body = "name=Alpha%20Centauri&email=alphacentauri%40smail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(2)
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body.into()).await;
    test_app.post_subscriptions(body.into()).await;
}
