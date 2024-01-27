use actix_server::run;
use std::net::TcpListener;

#[tokio::main]
pub async fn main() -> std::io::Result<()> {
    let listener =
        TcpListener::bind("127.0.0.1:8000").expect("failed to bind application listener");
    run(listener)?.await
}
