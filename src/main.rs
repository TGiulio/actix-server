use actix_server::configuration::get_configuration;
use actix_server::startup::run;
use actix_server::telemetry::{get_tracing_subscriber, init_tracing_subscriber};
use sqlx::PgPool;
use std::net::TcpListener;

#[tokio::main]
pub async fn main() -> std::io::Result<()> {
    //----------------------------- LOG SETTINGS ------------------------------------------------------------------------------
    let tracing_subscriber = get_tracing_subscriber(
        "actix_server".to_string(),
        "info".to_string(),
        std::io::stdout,
    );
    init_tracing_subscriber(tracing_subscriber);
    //----------------------------- END LOG SETTINGS ------------------------------------------------------------------------------

    // read configuration
    let configuration = get_configuration().expect("failed to load configuration");
    // get database connection
    let db_connection_pool = PgPool::connect(&configuration.database.connection_string("require"))
        .await
        .expect("failed to connect to the database");
    // set the address
    let address = format!("127.0.0.1:{}", configuration.application_port);
    // start listener
    let listener = TcpListener::bind(address).expect("failed to bind application listener");
    run(listener, db_connection_pool)?.await
}
