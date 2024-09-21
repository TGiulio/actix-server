use actix_server::startup::Application;
use actix_server::{
    configuration::{get_configuration, DatabaseSettings},
    startup::get_connection_pool,
    telemetry::{get_tracing_subscriber, init_tracing_subscriber},
};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;

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
    pub email_server: MockServer,
    pub port: u16,
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

pub struct RevocationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "Application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("couldn't send the request.")
    }

    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let request_body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        // declare closure to find the links in a string
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 2); // confirmation and revocation link!
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            // check that we don't call someone else's API
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            // set the port, only for testing purposes, not needed in production
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(&request_body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(&request_body["TextBody"].as_str().unwrap());

        ConfirmationLinks { html, plain_text }
    }

    pub fn get_revocation_links(&self, email_request: &wiremock::Request) -> RevocationLinks {
        let request_body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        // declare closure to find the links in a string
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 2); // confirmation and revocation link!
            let raw_link = links[1].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            // check that we don't call someone else's API
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            // set the port, only for testing purposes, not needed in production
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(&request_body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(&request_body["TextBody"].as_str().unwrap());

        RevocationLinks { html, plain_text }
    }
}

pub async fn spawn_app() -> TestApp {
    // LOG INITIALIZATION
    // use environment variable TEST_LOG = true to display the log messages
    Lazy::force(&TRACING);
    // END LOG INITIALIZATION

    // let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind random port");
    // let port = listener.local_addr().unwrap().port();
    let email_server = MockServer::start().await;

    //get configuration
    let configuration = {
        let mut c = get_configuration().expect("failed to load configuration");
        // twist the database name to work outside the production database
        c.database.database_name = Uuid::new_v4().to_string();
        // use mock server as email API
        c.email_client.base_url = email_server.uri();
        // use a random port
        c.application.port = 0;
        c
    };

    //configure database pool
    configure_test_database(&configuration.database).await;

    let application = Application::build(configuration.clone())
        .await
        .expect("failed to build the application");
    let application_port = application.port();
    let address = format!("http://127.0.0.1:{}", application_port);
    let _ = tokio::spawn(application.run_until_stopped());

    TestApp {
        address,
        db_pool: get_connection_pool(&configuration.database).await,
        email_server,
        port: application_port,
    }
}

async fn configure_test_database(db_conf: &DatabaseSettings) -> PgPool {
    // connect to the database
    let mut connection = PgConnection::connect_with(&db_conf.without_db())
        .await
        .expect("failed to connect to the database");

    // create the database to use
    connection
        .execute(format!(r#"CREATE DATABASE "{}""#, db_conf.database_name).as_str())
        .await
        .expect("failed to create database");

    //create the pool and execute migrations
    let db_pool = PgPool::connect_with(db_conf.with_db())
        .await
        .expect("failed to connect to the newly created database");
    //
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("failed do execute migrations");
    db_pool
}
