use const_format::concatcp;
use reqwest;

use super::Node;

const PORT: u16 = 30000;
const BASE_URL: &str = concatcp!("http://localhost:", PORT);

pub async fn add_node(node: &Node) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let _ = client
        .post(concatcp!(BASE_URL, "/add_node"))
        .json(&node)
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

pub async fn get_nodes() -> anyhow::Result<Vec<Node>> {
    // Making GET request to /get_nodes endpoint
    let client = reqwest::Client::new();
    let response = client
        .get(concatcp!(BASE_URL, "/get_nodes"))
        .send()
        .await?
        .error_for_status()?;

    let nodes: Vec<Node> = response.json().await?;

    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;

    #[tokio::test]
    async fn add_nodes() -> anyhow::Result<()> {
        let node = Node {
            ip: Ipv4Addr::new(127, 0, 0, 1),
            port: 123,
        };

        add_node(&node).await?;
        let nodes = get_nodes().await?;

        assert!(nodes.contains(&node));
        Ok(())
    }
}
