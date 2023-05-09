use std::env;

// Usage example:
use ctrader_fix::FixApi;

#[async_std::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let host = env::var("CTRADER_FIX_HOST").unwrap();
    let username = env::var("CTRADER_FIX_USERNAME").unwrap();
    let password = env::var("CTRADER_FIX_PASSWORD").unwrap();
    let broker = env::var("CTRADER_FIX_BROKER").unwrap();

    let mut fix = FixApi::new(host, username, password, broker, None);
    fix.connect().await?;

    fix.logon().await?;
    log::info!("sent logon");
    async_std::task::sleep(std::time::Duration::from_secs(2)).await;

    fix.logout().await?;
    log::info!("sent logout");
    async_std::task::sleep(std::time::Duration::from_secs(2)).await;

    fix.disconnect().await?;
    log::info!("sent disconnect");
    async_std::task::sleep(std::time::Duration::from_secs(1)).await;

    Ok(())
}
