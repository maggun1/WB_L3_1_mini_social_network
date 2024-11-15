mod auth;
mod db;
mod handlers;
mod models;

use axum::{
    routing::{post, get, delete},
    Router,
};

use tokio_postgres::NoTls;

use crate::handlers::{
    register,
    login,
    create_post,
    get_post,
    delete_post,
    like_post
};
use crate::db::Database;

#[tokio::main]
async fn main() {
    let (client, connection) = tokio_postgres::connect(
        "postgres://wb:wb@localhost/wb_db",
        NoTls,
    ).await.unwrap();

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let db = Database::new(client);
    db.init().await.unwrap();

    let app = Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/posts", post(create_post))
        .route("/posts/:id", get(get_post))
        .route("/posts/:id", delete(delete_post))
        .route("/posts/:id/likes", post(like_post))
        .with_state(db.clone());

    let addr = "127.0.0.1:3000";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Mini social network server started at http://{}", addr);
    axum::serve(listener, app).await.unwrap();
}