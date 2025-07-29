use axum::{
    Router,
    extract::Json,
    http::{Method, StatusCode},
    response::Json as ResponseJson,
    routing::post,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Deserialize)]
struct LogMessage {
    level: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct LogResponse {
    status: String,
}

async fn receive_logs(
    Json(payload): Json<LogMessage>,
) -> Result<ResponseJson<LogResponse>, StatusCode> {
    // Log using tracing with simplified format (no target, module_path, or location)
    match payload.level.to_lowercase().as_str() {
        "error" => error!("{}", payload.message),
        "warn" | "warning" => warn!("{}", payload.message),
        "info" => info!("{}", payload.message),
        "debug" => debug!("{}", payload.message),
        "trace" => trace!("{}", payload.message),
        _ => info!("{}", payload.message),
    }

    Ok(ResponseJson(LogResponse {
        status: "received".to_string(),
    }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Find the project root by looking for Cargo.toml
    let current_dir = std::env::current_dir()?;
    let project_root = current_dir
        .ancestors()
        .find(|path| path.join("Cargo.toml").exists())
        .ok_or_else(|| anyhow::anyhow!("Could not find project root"))?;

    let tmp_dir = project_root.join("tmp");
    std::fs::create_dir_all(&tmp_dir)?;

    // Create a simple file appender that truncates on each start
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(tmp_dir.join("dev-log-server.log"))?;
    let (non_blocking_appender, _guard) = tracing_appender::non_blocking(log_file);

    // Configure file layer
    let file_layer = fmt::layer()
        .with_writer(non_blocking_appender)
        .with_ansi(false) // No colors in file
        .with_target(false) // Remove target from log format
        .with_thread_ids(false) // Remove thread ids from log format
        .with_thread_names(false) // Remove thread names from log format
        .without_time(); // Remove timestamp from log format

    // Configure stdout layer with custom formatting
    let stdout_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(true)
        .with_ansi(true) // Enable colors for stdout
        .compact();

    // Initialize tracing with multiple layers
    tracing_subscriber::registry()
        .with(file_layer)
        .with(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "dev_log_server=info,tower_http=info,dissonance_lab=debug".into()
            }),
        )
        .with(stdout_layer)
        .init();

    // Build CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    // Build the application router
    let app = Router::new()
        .route("/logs", post(receive_logs))
        .layer(ServiceBuilder::new().layer(cors).into_inner());

    // Configure the server address
    let port = std::env::var("DEV_LOG_SERVER_PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()?;

    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    info!(target: "dev_log_server", "Log server starting on http://{}", addr);
    info!(target: "dev_log_server", "Ready to receive logs from /logs");

    // Start the server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    // The _guard must be kept in scope until the end of main
    drop(_guard);

    Ok(())
}
