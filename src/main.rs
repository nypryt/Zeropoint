use zeropoint::ZeroPoint;
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    let zeropoint = ZeroPoint::new().await?;
    zeropoint.run().await?;
    
    Ok(())
}