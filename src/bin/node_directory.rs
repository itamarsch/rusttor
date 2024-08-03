use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use const_format::concatcp;
use env_logger::Env;
use log::info;
use serde::Deserialize;
use std::{collections::BTreeMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    net::TcpStream,
    sync::Mutex,
    time::{interval, timeout},
};

const PORT: u16 = 30000;

type Valid = bool;
#[derive(Default)]
struct AppState {
    nodes: tokio::sync::RwLock<BTreeMap<SocketAddr, Mutex<Valid>>>,
}

#[derive(Deserialize)]
struct GetNodesQuery {
    amount: Option<usize>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    let app_state = Arc::new(AppState::default());
    let app_state_for_task = app_state.clone();
    let data = web::Data::new(app_state);

    // Clone the app state for the background task

    // Spawn a background task to invalidate/update the app state
    tokio::task::spawn(async move {
        let mut interval = interval(Duration::from_secs(5));
        loop {
            println!("Invalidating");
            interval.tick().await;
            let nodes = app_state_for_task.nodes.read().await;
            for (node, is_valid) in &*nodes {
                if let Ok(Ok(_)) =
                    timeout(Duration::from_secs_f32(3.0), TcpStream::connect(node)).await
                {
                    let mut is_valid_guard = is_valid.lock().await;
                    println!("Valid node: {:?}", node);
                    *is_valid_guard = true;
                    drop(is_valid_guard)
                } else {
                    let mut is_valid_guard = is_valid.lock().await;
                    println!("Invalid node: {:?}", node);
                    *is_valid_guard = false;
                    drop(is_valid_guard)
                };
            }
        }
    });

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

async fn add_node(data: web::Data<Arc<AppState>>, node: web::Json<SocketAddr>) -> impl Responder {
    let nodes = &mut *data.nodes.write().await;
    nodes.insert(node.into_inner(), Mutex::new(false));

    HttpResponse::Ok().body("Node added")
}

async fn get_nodes(
    data: web::Data<Arc<AppState>>,
    query: web::Query<GetNodesQuery>,
) -> impl Responder {
    let nodes = &*data.nodes.read().await;
    let amount = query.amount.unwrap_or(5);
    let mut valid_nodes = vec![];
    for (node, is_valid) in nodes.iter().take(amount) {
        let is_valid = *is_valid.lock().await;
        if is_valid {
            valid_nodes.push(node)
        }
    }

    HttpResponse::Ok().json(&valid_nodes)
}
