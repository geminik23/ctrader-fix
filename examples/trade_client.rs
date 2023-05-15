use cfix::{
    types::{ConnectionHandler, Side},
    TradeClient,
};
use std::{env, error::Error, sync::Arc};

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
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let host = env::var("CTRADER_FIX_HOST").unwrap();
    let username = env::var("CTRADER_FIX_USERNAME").unwrap();
    let password = env::var("CTRADER_FIX_PASSWORD").unwrap();
    let sender_comp_id = env::var("CTRADER_FIX_SENDERCOMPID").unwrap();

    let handler = Arc::new(Handler {});
    let mut client = TradeClient::new(host, username, password, sender_comp_id, None);
    client.register_connection_handler_arc(handler.clone());

    // connect and logon
    client.connect().await?;
    if client.is_connected() {
        let res = client.fetch_security_list().await?;
        log::info!("Secutiry list - {:?}", res);

        // //
        // log::info!("Request fetch positions");
        // let res = client.fetch_positions().await?;
        // log::info!("Positions - {:?}", res);

        //
        log::info!("Request fetch order mass");
        let res = client.fetch_all_order_status(None).await?;
        log::info!("Order mass - {:?}", res);

        // log::info!("New market order");
        // let res = client
        //     .new_market_order(1, Side::BUY, 1000.0, None, None, None, None)
        //     .await?;
        // log::info!("Result of market order - {:?}", res);

        // log::info!("New limit order");
        // let res = client
        //     .new_limit_order(1, Side::BUY, 0.9, 1000.0, None, None, None, None, None)
        //     .await?;
        // log::info!("Result of limit order - {:?}", res);

        // log::info!("New stop order");
        // let res = client
        //     .new_stop_order(1, Side::SELL, 1.1, 1000.0, None, None, None, None, None)
        //     .await?;
        // log::info!("Result of stop order - {:?}", res);

        async_std::task::sleep(std::time::Duration::from_secs(5)).await;
    }

    // disconnect
    client.disconnect().await?;
    async_std::task::sleep(std::time::Duration::from_secs(2)).await;

    Ok(())
}
