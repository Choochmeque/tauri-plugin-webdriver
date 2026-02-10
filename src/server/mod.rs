use std::net::SocketAddr;
use std::sync::Arc;

use tauri::{AppHandle, Runtime};
use tokio::runtime::Runtime as TokioRuntime;
use tokio::sync::RwLock;

pub mod handlers;
pub mod response;
pub mod router;

use crate::webdriver::SessionManager;

/// Shared state for the WebDriver server
pub struct AppState<R: Runtime> {
    pub app: AppHandle<R>,
    pub sessions: RwLock<SessionManager>,
}

impl<R: Runtime> AppState<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self {
            app,
            sessions: RwLock::new(SessionManager::new()),
        }
    }
}

/// Start the WebDriver HTTP server on the specified port
pub fn start<R: Runtime + 'static>(app: AppHandle<R>, port: u16) {
    std::thread::spawn(move || {
        let rt = TokioRuntime::new().expect("Failed to create Tokio runtime");

        rt.block_on(async {
            let state = Arc::new(AppState::new(app));
            let router = router::create_router(state);

            let addr = SocketAddr::from(([127, 0, 0, 1], port));

            tracing::info!("WebDriver server listening on http://{}", addr);

            let listener = tokio::net::TcpListener::bind(addr)
                .await
                .expect("Failed to bind to address");

            axum::serve(listener, router)
                .await
                .expect("Server error");
        });
    });
}
