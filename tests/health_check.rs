use actix_server::{
    configuration::{get_configuration, DatabaseSettings},
    startup::run,
    telemetry::{get_tracing_subscriber, init_tracing_subscriber},
};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;

static TRACING: Lazy<()> = Lazy::new(|| {
    let sub_name = "test_actix_server".to_string();
    let sub_env_filter = "debug".to_string();

    // use environment variable TEST_LOG = true to display the log messages
    if std::env::var("TEST_LOG").is_ok() {
        let tracing_subscriber = get_tracing_subscriber(sub_name, sub_env_filter, std::io::stdout);
        init_tracing_subscriber(tracing_subscriber);
    } else {
        let tracing_subscriber = get_tracing_subscriber(sub_name, sub_env_filter, std::io::sink);
        init_tracing_subscriber(tracing_subscriber);
    }
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub db_conf: DatabaseSettings,
}

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

async fn configure_test_database(db_conf: &DatabaseSettings) -> PgPool {
    // connect to the database
    let mut connection = PgConnection::connect(&db_conf.connection_string_without_db("require"))
        .await
        .expect("failed to connect to the database");

    // create the database to use
    connection
        .execute(format!(r#"CREATE DATABASE "{}""#, db_conf.database_name).as_str())
        .await
        .expect("failed to create database");

    //create the pool and execute migrations
    let db_pool = PgPool::connect(&db_conf.connection_string("require"))
        .await
        .expect("failed to connect to the newly created database");
    //
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("failed do execute migrations");
    db_pool
}

async fn spawn_app() -> TestApp {
    // LOG INITIALIZATION
    // use environment variable TEST_LOG = true to display the log messages
    Lazy::force(&TRACING);
    // END LOG INITIALIZATION

    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    //get configuration
    let mut configuration = get_configuration().expect("failed to load configuration");
    // twist the database name to work outside the production database
    configuration.database.database_name = Uuid::new_v4().to_string();
    //get database pool
    let db_connection_pool = configure_test_database(&configuration.database).await;

    let server = run(listener, db_connection_pool.clone()).expect("failed to bind address");

    let _ = tokio::spawn(server);
    TestApp {
        address: format!("http://127.0.0.1:{}", port),
        db_pool: db_connection_pool,
        db_conf: configuration.database,
    }
}

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    // prepare what's needed
    let test_app = spawn_app().await;
    let address = test_app.address;
    let db_pool = test_app.db_pool;

    let client = reqwest::Client::new();

    // make the request
    let body = "name=Alpha%20Centauri&email=alphacentauri%40smail.com";
    let response = client
        .post(&format!("{}/subscriptions", address))
        .header("Content-Type", "Application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("couldn't send the request.");

    // verify
    assert_eq!(response.status().as_u16(), 200);

    let saved = sqlx::query!("SELECT email, name FROM public.subscriptions")
        .fetch_one(&db_pool)
        .await
        .expect("cannot retrieve subscriber");

    assert_eq!(saved.email, "alphacentauri@smail.com");
    assert_eq!(saved.name, "Alpha Centauri");
}

#[tokio::test]
async fn subscribe_returns_400_for_missing_form_data() {
    // prepare what's needed
    let test_app = spawn_app().await;
    let address = test_app.address;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=Alpha%20Centauri", "missing email"),
        ("name=alphacentauri%40smail.com", "missing name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", address))
            .header("Content-Type", "Application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("couldn't send the request.");

        assert_eq!(
            response.status().as_u16(),
            400,
            "Tha API did not fail with 400 code when the body was {}",
            error_message
        );
    }
}
