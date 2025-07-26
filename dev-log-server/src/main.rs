use axum::{
    extract::Json,
    http::{Method, StatusCode},
    response::Json as ResponseJson,
    routing::post,
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn, error, debug, trace};

#[derive(Debug, Deserialize)]
struct LogMessage {
    level: String,
    message: String,
    target: Option<String>,
    timestamp: Option<DateTime<Utc>>,
    module_path: Option<String>,
    file: Option<String>,
    line: Option<u32>,
}

#[derive(Debug, Serialize)]
struct LogResponse {
    status: String,
}

async fn receive_logs(Json(payload): Json<LogMessage>) -> Result<ResponseJson<LogResponse>, StatusCode> {
    let timestamp = payload.timestamp.unwrap_or_else(Utc::now);
    let target = payload.target.as_deref().unwrap_or("frontend");
    
    // Format the log message for terminal display
    let location = match (payload.file.as_ref(), payload.line) {
        (Some(file), Some(line)) => format!(" ({}:{})", file, line),
        _ => String::new(),
    };
    
    let module = payload.module_path
        .as_ref()
        .map(|m| format!(" [{}]", m))
        .unwrap_or_default();
    
    let formatted_message = format!(
        "{} [{}{}{}] {}: {}",
        timestamp.format("%H:%M:%S%.3f"),
        payload.level.to_uppercase(),
        module,
        location,
        target,
        payload.message
    );
    
    // Log the message using tracing based on the level
    match payload.level.to_lowercase().as_str() {
        "error" => error!("{}", formatted_message),
        "warn" | "warning" => warn!("{}", formatted_message),
        "info" => info!("{}", formatted_message),
        "debug" => debug!("{}", formatted_message),
        "trace" => trace!("{}", formatted_message),
        _ => info!("{}", formatted_message),
    }
    
    Ok(ResponseJson(LogResponse {
        status: "received".to_string(),
    }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dev_log_server=info,tower_http=debug".into())
        )
        .init();

    // Build CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    // Build the application router
    let app = Router::new()
        .route("/logs", post(receive_logs))
        .layer(
            ServiceBuilder::new()
                .layer(cors)
                .into_inner(),
        );

    // Configure the server address
    let port = std::env::var("DEV_LOG_SERVER_PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()?;
    
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    
    info!("Development log server starting on http://{}", addr);
    info!("Ready to receive logs from frontend at POST /logs");

    // Start the server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
