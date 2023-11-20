/// The front service:
/// Accepts request from tcp port:
/// 1. assign a unique request id
/// 2. accordingly to conf file:
///   * rely request to backend (executor backend)
/// 
/// Accept command from other "actors"
/// (the only actor is the executor backserv):
/// 1. match the unique request id
/// 2. send back the payload received as a response to request_id
/// 
/// Problems:
/// - the frontservice callback synchronize with backserv: it waits until the corresponding response is ready.
/// - the backserv synchronize with the frontserv: a message sent to backend is matched with a waiting frontserv's message.
/// 
/// There could be an arbiter in the middle:
///  - the arbiter provide a channel to frontserv
///  - the arbiter store the request_id associated with the channel (is it possible to store a rx in a hashmap? Maybe no, but it is possible to store rx in array?)
///  - the arbiter: 1. provide feedback to backserv, 2. send back response to frontserv, 3. dealloc/close the channel for synchronization
///  - the arbiter manage a timeout on the request, and return a standard reply
/// 

struct FrontServ {
    
}

use axum::{
    error_handling::HandleErrorLayer,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    //time::Duration, simd::SimdConstPtr,
    net::SocketAddr
};
//use tower::{BoxError, ServiceBuilder};
//use tower_http::trace::TraceLayer;
//use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
// use uuid::Uuid;

use crate::arbiter::{ArbiterHandler, self};

/*
tracing_subscriber::registry()
.with(
    tracing_subscriber::EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| "example_todos=debug,tower_http=debug".into()),
)
.with(tracing_subscriber::fmt::layer())
.init();
*/
pub async fn run_front(arbiter: ArbiterHandler) {
    // let db = Db::default();

    // Compose the routes
    let app = Router::new()
        .route("/todos", get(todos_index))
        .with_state(arbiter);
        /*
        // Add middleware to all routes
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|error: BoxError| async move {
                    if error.is::<tower::timeout::error::Elapsed>() {
                        Ok(StatusCode::REQUEST_TIMEOUT)
                    } else {
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Unhandled internal error: {error}"),
                        ))
                    }
                }))
                .timeout(Duration::from_secs(10))
                .layer(TraceLayer::new_for_http())
                .into_inner(),
        )
        */
    
    /*
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
    .await
    .unwrap();
    */
    //tracing::debug!("listening on {}", listener.local_addr().unwrap());
    //axum::serve(listener, app).await.unwrap();
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    axum::Server::bind(&addr)
    .serve(app.into_make_service())
    .await
    .unwrap();
}



async fn todos_index(
    State(arbiter): State<ArbiterHandler>,
) -> impl IntoResponse {
    //let todos = db.read().unwrap();

    //let Query(pagination) = pagination.unwrap_or_default();
    
    Json(true)
    //let todos = todos
    //    .values()
    //    //.skip(pagination.offset.unwrap_or(0))
    //    //.take(pagination.limit.unwrap_or(usize::MAX))
    //    .cloned()
    //    .collect::<Vec<_>>();
//
    //Json(todos)
}
