use echodorsal_alexa::validation::{Allowlist, Validator};
use serde::Deserialize;
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
};

#[derive(Deserialize)]
struct Config {
    server: Server,
    alexa: Allowlist,
}
#[derive(Deserialize)]
struct Server {
    host: String,
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Missing config, empty allowlists, and non-loopback bind requests fail startup.
    let text = tokio::fs::read_to_string("echodorsal.toml").await?;
    let config: Config = toml::from_str(&text)?;
    anyhow::ensure!(
        config.server.host == "127.0.0.1",
        "server.host must be 127.0.0.1"
    );
    let validator = Arc::new(Validator::new(config.alexa)?);
    let listener =
        tokio::net::TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, config.server.port))
            .await?;
    axum::serve(listener, echodorsal_server::router(validator)).await?;
    Ok(())
}
