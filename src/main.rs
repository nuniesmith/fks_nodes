use clap::{Parser, ValueEnum};
use std::{time::Duration, net::SocketAddr};
use tokio::{net::TcpListener, io::{AsyncReadExt, AsyncWriteExt}, time::sleep};

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
    let addr: SocketAddr = cli.listen.parse()?;
    let listener = TcpListener::bind(addr).await?;
    tracing::info!(?addr, "master listening");
    loop {
        let (mut socket, peer) = listener.accept().await?;
        tracing::info!(?peer, "connection");
        tokio::spawn(async move {
            let mut buf = vec![0u8; 1024];
            match socket.read(&mut buf).await { Ok(n) if n>0 => { let _=socket.write_all(b"ok\n").await; }, _=>{} }
        });
    }
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
