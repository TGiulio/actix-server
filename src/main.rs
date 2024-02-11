use actix_server::configuration::get_configuration;
use actix_server::startup::run;
use sqlx::PgPool;
use std::net::TcpListener;

#[tokio::main]
pub async fn main() -> std::io::Result<()> {
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
