use cfix::{
    types::{ConnectionHandler, ExecutionReport, Side, TradeDataHandler},
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

#[async_trait::async_trait]
impl TradeDataHandler for Handler {
    async fn on_execution_report(&self, exec_report: ExecutionReport) {
        log::info!("on execution repost : {:?}", exec_report);
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
    let sender_comp_id = env::var("CTRADER_FIX_SENDERCOMPID").unwrap();

    let handler = Arc::new(Handler {});
    let mut client = TradeClient::new(host, username, password, sender_comp_id, None);
    client.register_connection_handler_arc(handler.clone());
    client.register_trade_handler_arc(handler.clone());

    // connect and logon
    client.connect().await?;
    if client.is_connected() {
        let res = client.fetch_security_list().await?;
        for symbolinfo in res.into_iter() {
            println!("{:?}", symbolinfo);
        }
        client.disconnect().await?;
    }

    Ok(())
}
