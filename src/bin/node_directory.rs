use std::collections::BTreeSet;

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use const_format::concatcp;
use rustor::tor::Node;
use tokio::sync::Mutex;

const PORT: u16 = 30000;

#[derive(Default)]
struct AppState {
    nodes: Mutex<BTreeSet<Node>>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let data = web::Data::new(AppState::default());

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .route("/add_node", web::post().to(add_node))
            .route("/get_nodes", web::get().to(get_nodes))
    })
    .bind(concatcp!("0.0.0.0:", PORT))?
    .run()
    .await
}

async fn add_node(data: web::Data<AppState>, node: web::Json<Node>) -> impl Responder {
    let mut nodes = data.nodes.lock().await;
    nodes.insert(node.into_inner());
    HttpResponse::Ok().body("Node added")
}

async fn get_nodes(data: web::Data<AppState>) -> impl Responder {
    let nodes = data.nodes.lock().await;
    HttpResponse::Ok().json(&*nodes)
}
