#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Testing db here!");
    /* let _pool = SqlitePoolOptions::new()
    .max_connections(4)
    .connect_lazy("sqlite://server.db")?;*/
    Ok(())
}
