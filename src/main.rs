use actix_server::configuration::get_configuration;
use actix_server::startup::run;
use actix_server::telemetry::{get_tracing_subscriber, init_tracing_subscriber};
use sqlx::postgres::PgPoolOptions;
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
    let db_connection_pool = PgPoolOptions::new()
        .connect_timeout(std::time::Duration::from_secs(10))
        .connect_with(configuration.database.with_db())
        .await
        .expect("failed to connect to the database");
    // set the address
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    // start listener
    let listener = TcpListener::bind(address).expect("failed to bind application listener");
    run(listener, db_connection_pool)?.await
}
