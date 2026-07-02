use axum::Router;
use std::sync::Arc;
use tokio::net::TcpListener;

mod routes;
pub use routes::AppState;

pub struct HttpServer {
    port: u16,
}

impl HttpServer {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    pub async fn start(self, state: AppState) -> Result<(), Box<dyn std::error::Error>> {
        let app = routes::create_router(state);

        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;

        println!("HTTP server listening on {}", addr);

        axum::serve(listener, app).await?;

        Ok(())
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}
