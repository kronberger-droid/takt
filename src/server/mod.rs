//! HTTP server for `takt serve`. Wraps a `SqliteStore` and exposes the
//! `Store` operations as JSON endpoints. All request-handling code lives
//! here; the CLI in `main.rs` just spins up a runtime and calls `run`.

use std::{
    net::SocketAddr,
    path::Path,
    sync::{Arc, Mutex},
};

use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::Serialize;

use crate::{error::TaktError, store::{SqliteStore, Store}};

type SharedStore = Arc<Mutex<SqliteStore>>;

// TODO: define StatusResponse fields (what JSON shape should GET /status return?)
#[derive(Serialize)]
struct StatusResponse {
}

/// Start an HTTP server on `addr`, backed by a SQLite database at `db_path`.
/// Blocks until shutdown (Ctrl+C / SIGTERM).
pub async fn run(addr: SocketAddr, db_path: &Path) -> Result<(), TaktError> {
    let store = SqliteStore::open(db_path, 1)?;
    store.ensure_default_user()?;
    let shared: SharedStore = Arc::new(Mutex::new(store));

    let app = Router::new()
        .route("/status", get(status_handler))
        .with_state(shared);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("takt serve: listening on {addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn status_handler(
    State(_store): State<SharedStore>,
) -> Result<Json<StatusResponse>, StatusCode> {
    // TODO: query _store.lock() and build a StatusResponse
    todo!()
}

/// Resolves when the user sends Ctrl+C or the process receives SIGTERM.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    println!("takt serve: shutting down");
}
