use crate::helpers::spawn_app;

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    // prepare what's needed
    let test_app = spawn_app().await;
    let db_pool = &test_app.db_pool;

    // make the request
    let body = "name=Alpha%20Centauri&email=alphacentauri%40smail.com";
    let response = test_app.post_subscriptions(body.into()).await;

    // verify
    assert_eq!(response.status().as_u16(), 200);

    let saved = sqlx::query!("SELECT email, name FROM public.subscriptions")
        .fetch_one(db_pool)
        .await
        .expect("cannot retrieve subscriber");

    assert_eq!(saved.email, "alphacentauri@smail.com");
    assert_eq!(saved.name, "Alpha Centauri");
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
            "Tha API did not fail with 400 code when the body was {}",
            error_message
        );
    }
}
