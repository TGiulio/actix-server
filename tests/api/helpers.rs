use actix_server::startup::Application;
use actix_server::{
    configuration::{get_configuration, DatabaseSettings},
    startup::get_connection_pool,
    telemetry::{get_tracing_subscriber, init_tracing_subscriber},
};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
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
}

pub async fn spawn_app() -> TestApp {
    // LOG INITIALIZATION
    // use environment variable TEST_LOG = true to display the log messages
    Lazy::force(&TRACING);
    // END LOG INITIALIZATION

    // let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind random port");
    // let port = listener.local_addr().unwrap().port();

    //get configuration
    let configuration = {
        let mut c = get_configuration().expect("failed to load configuration");
        // twist the database name to work outside the production database
        c.database.database_name = Uuid::new_v4().to_string();
        // use a random port
        c.application.port = 0;
        c
    };

    //configure database pool
    configure_test_database(&configuration.database).await;

    let application = Application::build(configuration.clone())
        .await
        .expect("failed to build the application");
    let address = format!("http://127.0.0.1:{}", application.port());
    let _ = tokio::spawn(application.run_until_stopped());

    TestApp {
        address,
        db_pool: get_connection_pool(&configuration.database).await,
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
