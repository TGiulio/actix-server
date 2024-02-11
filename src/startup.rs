use crate::routes::{health_check, subscribe};
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;

pub fn run(listener: TcpListener, db_connection_pool: PgPool) -> Result<Server, std::io::Error> {
    // wrap the connection into an Arc (smart pointer) so we can clone it inside the closure
    //web::Data is another extractor that returns an Arc
    let db_pool = web::Data::new(db_connection_pool);
    //
    let server = HttpServer::new(move || {
        App::new()
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(db_pool.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
