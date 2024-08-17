use actix_server::configuration::get_configuration;
use actix_server::email_client::{self, EmailClient};
use actix_server::startup::Application;
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

    //build the application
    let application = Application::build(configuration).await?;
    application.run_until_stopped().await?;

    Ok(())
}
