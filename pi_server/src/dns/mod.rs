use crate::dns::resolver::CloudFrontAuthority;
use tokio::net::{TcpListener, UdpSocket};
use tokio::time::Duration;
use trust_dns_server::authority::{Authority, Catalog};
use trust_dns_server::ServerFuture;

mod resolver;

pub async fn start_dns_server(port: u64) -> anyhow::Result<()> {
    log::debug!("Starting DNS TCP/UDP server at port {}", port);

    let mut handler = Catalog::default();
    let authority = CloudFrontAuthority::default()?;
    handler.upsert(authority.origin().clone(), authority.into_authority_obj());

    let address = format!("0.0.0.0:{}", port);
    let mut server = ServerFuture::new(handler);
    server.register_socket(UdpSocket::bind(&address).await?);
    server.register_listener(
        TcpListener::bind(address).await?,
        Duration::from_millis(200),
    );
    Ok(server.block_until_done().await?)
}
