//! HTTP server for `takt serve`. Wraps a `SqliteStore` and exposes the
//! `Store` operations as JSON endpoints. All request-handling code lives
//! here; the CLI in `main.rs` just spins up a runtime and calls `run`.
//!
//! Phase 2 of v0.3 will add:
//!   - Handlers for every `Store` verb under `/start`, `/stop`, `/status`,
//!     `/report`, `/tags`.
//!   - Bearer-token middleware that resolves a `user_id` from the DB and
//!     attaches it to request extensions.
//!   - `Arc<Mutex<SqliteStore>>` as shared state (single-writer is fine for
//!     v0.3's personal-server scale).

use std::net::SocketAddr;

use axum::Router;

use crate::error::TaktError;

/// Start an HTTP server on `addr` and block until shutdown.
/// Currently an empty router — `/` returns 404, nothing else is wired up.
pub async fn run(addr: SocketAddr) -> Result<(), TaktError> {
    let app = Router::new();

    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("takt serve: listening on {addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
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
