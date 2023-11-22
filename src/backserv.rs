/// The backserv listen on unix socket, as
/// specified in the config file
/// 


use axum::{
    error_handling::HandleErrorLayer,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::join;
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

use crate::arbiter::{ArbiterHandler, self, ForHttpResponse};

/*
tracing_subscriber::registry()
.with(
    tracing_subscriber::EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| "example_todos=debug,tower_http=debug".into()),
)
.with(tracing_subscriber::fmt::layer())
.init();
*/

use hyperlocal::UnixServerExt;

pub async fn run_backserv(socketpath: &str, arbiter: ArbiterHandler) {
    let path = std::path::Path::new(socketpath);

    if path.exists() {
        tokio::fs::remove_file(path).await.expect("Could not remove old socket!");
    }

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
    axum::Server::bind_unix(&path)
    .expect("cannot open unix socket")
    .serve(app.into_make_service())
    .await
    .unwrap();
}



async fn todos_index(
    State(arbiter): State<ArbiterHandler>,
) -> impl IntoResponse {
    //let todos = db.read().unwrap();

    //let Query(pagination) = pagination.unwrap_or_default();
    arbiter.fulfill_request("123", ForHttpResponse{
        code: 123,
        data: serde_json::Value::Bool(true)
    }).await.unwrap();
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
