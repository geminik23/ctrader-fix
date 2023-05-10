use std::{env, error::Error};

use cfix::{ConnectionHandler, MarketClient};

// Usage example:
//

struct Handler;

#[async_trait::async_trait]
impl ConnectionHandler for Handler {
    async fn on_connect(&self) {
        log::info!("in handler : connected");
    }
    async fn on_logon(&self) {
        log::info!("in handler : logon");
    }
    async fn on_disconnect(&self) {
        log::info!("in handler : disconnected");
    }
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();
    // env_logger::init();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    let host = env::var("CTRADER_FIX_HOST").unwrap();
    let username = env::var("CTRADER_FIX_USERNAME").unwrap();
    let password = env::var("CTRADER_FIX_PASSWORD").unwrap();
    let broker = env::var("CTRADER_FIX_BROKER").unwrap();

    let handler = Handler {};
    let mut client = MarketClient::new(host, username, password, broker, None);
    client.register_connection_handler(handler);

    // connect and logon
    client.connect().await?;
    if client.is_connected() {
        client.subscribe(11).await?;

        async_std::task::sleep(std::time::Duration::from_secs(5)).await;
        //
    }

    // disconnect
    client.disconnect().await?;
    async_std::task::sleep(std::time::Duration::from_secs(2)).await;

    Ok(())
}
