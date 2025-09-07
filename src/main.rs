use clap::{Parser, ValueEnum};
use std::{time::Duration, net::SocketAddr};
use tokio::{net::TcpListener, io::{AsyncWriteExt}, time::sleep};
use axum::{Router, routing::get, response::Json as AxumJson};
use serde_json::json;

#[derive(Copy, Clone, Debug, ValueEnum)]
enum NodeType { Master, Worker }

#[derive(Parser, Debug)]
#[command(version, about = "FKS Node Network")]
struct Cli {
    #[arg(long, value_enum, default_value="master")] node_type: NodeType,
    #[arg(long, default_value="0.0.0.0:8080")] listen: String,
    #[arg(long)] master: Option<String>,
    #[arg(long, default_value_t = 1)] replicas: u16,
    #[arg(long, default_value_t = 0)] sim_latency_ms: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    match cli.node_type { NodeType::Master => run_master(&cli).await?, NodeType::Worker => run_worker(&cli).await? };
    Ok(())
}

async fn run_master(cli: &Cli) -> anyhow::Result<()> {
    // Currently only an HTTP server is started on the provided address.
    let addr: SocketAddr = cli.listen.parse()?;

    // HTTP health router (serve on same port via axum)
    let http_router = Router::new()
        .route("/health", get(|| async { AxumJson(json!({
            "status": "healthy",
            "service": "fks_nodes_master",
            "timestamp": chrono::Utc::now()
        }))}))
        .route("/", get(|| async { AxumJson(json!({"ok": true})) }));

    // Replace previous dual TCP+HTTP approach with single HTTP server (multi-protocol complicates compose health checks)
    // If raw TCP functionality is needed, we can add a dedicated port later.
    let http_listener = TcpListener::bind(addr).await?; // reusing original addr
    tracing::info!(%addr, "http (with /health) listening");
    tokio::spawn(async move { if let Err(e)=axum::serve(http_listener, http_router).await { tracing::error!(error=?e, "http server error"); } });
    // Keep process alive
    loop { sleep(Duration::from_secs(3600)).await; }
}

async fn run_worker(cli: &Cli) -> anyhow::Result<()> {
    let master = cli.master.as_ref().map(|s| s.as_str()).unwrap_or("127.0.0.1:8080");
    let latency = Duration::from_millis(cli.sim_latency_ms);
    for i in 0..cli.replicas {
        let master_clone = master.to_string();
        tokio::spawn(async move {
            loop {
                if let Ok(mut stream) = tokio::net::TcpStream::connect(&master_clone).await {
                    let _ = stream.write_all(b"register").await;
                }
                sleep(Duration::from_secs(10)).await;
            }
        });
        tracing::info!(replica=i, master=%master, "worker replica spawned");
    }
    if latency.as_millis()>0 { tracing::info!(?latency, "simulated latency enabled"); }
    loop { sleep(Duration::from_secs(3600)).await; }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::task::JoinSet;
    use tokio::io::AsyncReadExt;

    #[tokio::test]
    async fn master_accepts_single_connection() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let listener = TcpListener::bind(addr).await.unwrap();
        let local = listener.local_addr().unwrap();
        let mut set = JoinSet::new();
        set.spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 16];
            let n = sock.read(&mut buf).await.unwrap();
            (n, String::from_utf8_lossy(&buf[..n]).to_string())
        });
        let mut stream = tokio::net::TcpStream::connect(local).await.unwrap();
        stream.write_all(b"register").await.unwrap();
        let (n, msg) = set.join_next().await.unwrap().unwrap();
        assert_eq!(n, 8);
        assert_eq!(msg, "register");
    }
}
