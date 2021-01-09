use std::str::FromStr;
use std::thread;

use tokio::net::UdpSocket;
use tokio::time::{self, Duration};
use trust_dns_client::client::AsyncClient;
use trust_dns_client::client::ClientHandle;
use trust_dns_proto::rr::{DNSClass, Name, RecordType};
use trust_dns_proto::udp::UdpClientStream;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Hello World : {:?}", thread::current());

    let connection = UdpClientStream::<UdpSocket>::new("127.0.0.1:5053".parse()?);
    let (mut client, bg) = AsyncClient::connect(connection).await?;
    tokio::spawn(bg);

    for i in 0..10 {
        println!("{}. sending request...", i);
        if let Ok(result) = client
            .query(
                Name::from_str("www.amazon.in.")?,
                DNSClass::IN,
                RecordType::PTR,
            )
            .await
        {
            dbg!(result);
            time::sleep(Duration::from_secs(1)).await;
        } else {
            eprintln!("meh, failed");
        }
    }
    Ok(())
}
