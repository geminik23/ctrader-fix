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
        let symbol_id = 11;
        match client.subscribe_spot(symbol_id).await {
            Ok(_) => {
                log::info!("Success to spot subscribe the symbol_id({})", symbol_id);
                async_std::task::sleep(std::time::Duration::from_secs(5)).await;

                // try to subscription again
                // if let Err(err) = client.subscribe_spot(symbol_id).await {
                //     log::error!("{}", err);
                // }
                //
                log::info!(
                    "The prices of symbol_id({}) is {:?}",
                    symbol_id,
                    client.price_of(symbol_id).await?
                );

                log::info!(
                    "Spot subscription list : {:?}",
                    client.spot_subscription_list().await
                );

                client.unsubscribe_spot(symbol_id).await?;
                log::info!("Success to spot unsubscribe the symbol_id({})", symbol_id);

                // try to unsubscription again
                // if let Err(err) = client.unsubscribe_spot(symbol_id).await {
                //     log::error!("{}", err);
                // }
                //
                log::info!(
                    "Spot subscription list : {:?}",
                    client.spot_subscription_list().await
                );
            }
            Err(err) => {
                log::error!(
                    "Failed to subscribe the symbol_id({}) - {:?}",
                    symbol_id,
                    err
                );
            }
        }

        // depth market
        match client.subscribe_depth(symbol_id).await {
            Ok(_) => {
                log::info!("Success to depth subscribe the symbol_id({})", symbol_id);
                async_std::task::sleep(std::time::Duration::from_secs(5)).await;

                log::info!(
                    "The depth data of symbol_id({}) is {:?}",
                    symbol_id,
                    client.depth_data(symbol_id).await?
                );

                // try to subscription again
                // if let Err(err) = client.subscribe_depth(symbol_id).await {
                //     log::error!("{}", err);
                // }

                log::info!(
                    "Depth subscription list : {:?}",
                    client.depth_subscription_list().await
                );

                client.unsubscribe_depth(symbol_id).await?;
                log::info!("Success to depth unsubscribe the symbol_id({})", symbol_id);

                log::info!(
                    "Depth subscription list : {:?}",
                    client.depth_subscription_list().await
                );

                // try to unsubscription again
                // if let Err(err) = client.unsubscribe_depth(symbol_id).await {
                //     log::error!("{}", err);
                // }
            }
            Err(err) => {
                log::error!(
                    "Failed to subscribe the symbol_id({}) - {:?}",
                    symbol_id,
                    err
                );
            }
        }
    }

    // disconnect
    client.disconnect().await?;
    async_std::task::sleep(std::time::Duration::from_secs(2)).await;

    Ok(())
}
