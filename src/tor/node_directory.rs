use std::net::SocketAddr;

use const_format::concatcp;
use reqwest;

const PORT: u16 = 30000;
const BASE_URL: &str = concatcp!("http://localhost:", PORT);

pub async fn add_node(node: &SocketAddr) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let _ = client
        .post(concatcp!(BASE_URL, "/add_node"))
        .json(&node)
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

pub async fn get_nodes(n: u8) -> anyhow::Result<Vec<SocketAddr>> {
    // Making GET request to /get_nodes endpoint
    let client = reqwest::Client::new();
    let response = client
        .get(concatcp!(BASE_URL, "/get_nodes"))
        .query(&[("amount", n)])
        .send()
        .await?
        .error_for_status()?;

    let nodes: Vec<SocketAddr> = response.json().await?;

    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddrV4};

    use super::*;

    async fn add_nodes() -> anyhow::Result<()> {
        let node = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 123));

        add_node(&node).await?;
        let nodes = get_nodes(3).await?;

        assert!(!nodes.is_empty());
        Ok(())
    }
}
