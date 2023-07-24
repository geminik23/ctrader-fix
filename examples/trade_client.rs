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
        // log::info!("Secutiry list - {:?}", res);

        // POSITIONS
        {
            // fetch all positions
            log::info!("Request fetch positions");
            let res = client.fetch_positions().await?;
            log::info!("Positions - {:?}", res);
        }

        // ORDERS
        {
            // Fetch all orders
            // log::info!("Request fetch order mass");
            // let res = client.fetch_all_order_status(None).await?;
            // log::info!("Order mass - {:?}", res);
        }

        { // Test : market order & (add more lot & close partially & closs position) with change_position
             // log::info!("New market order");
             // let res = client
             //     .new_market_order(1, Side::BUY, 1000.0, None, None)
             //     .await?;
             // log::info!("Result of market order - {:?}", res);
             // async_std::task::sleep(std::time::Duration::from_secs(2)).await;
             //
             // let mut order_report = res.order_report;
             //
             // let lotsize = 2000.0;
             // log::info!("Add more lot {}", lotsize);
             // let res = client
             //     .adjust_position_size(
             //         order_report.pos_main_rept_id.clone(),
             //         order_report.symbol,
             //         lotsize,
             //         Side::BUY,
             //     )
             //     .await;
             // log::info!("Result of adjust position size - {:?}", res);
             // order_report.order_qty += lotsize;
             // async_std::task::sleep(std::time::Duration::from_secs(2)).await;
             //
             // let lotsize = 1000.0;
             // log::info!("Close position partially {}", lotsize);
             // let res = client
             //     .adjust_position_size(
             //         order_report.pos_main_rept_id.clone(),
             //         order_report.symbol,
             //         1000.0,
             //         Side::SELL,
             //     )
             //     .await;
             // log::info!("Result of close partially size - {:?}", res);
             // order_report.order_qty -= lotsize;
             // async_std::task::sleep(std::time::Duration::from_secs(2)).await;
             //
             // log::info!("Close position");
             // let res = client
             //     .adjust_position_size(
             //         order_report.pos_main_rept_id.clone(),
             //         order_report.symbol,
             //         order_report.order_qty,
             //         Side::SELL,
             //     )
             //     .await;
             // log::info!("Result of close position - {:?}", res);
        }

        { // Test : market order & get position report & close position
             // log::info!("New market order");
             // let order_res = client
             //     .new_market_order(1, Side::BUY, 1000.0, None, None)
             //     .await?;
             // log::info!("Result of market order - {:?}", res);
             // async_std::task::sleep(std::time::Duration::from_secs(2)).await;
             //
             // log::info!("Request fetch positions");
             // let res = client.fetch_positions().await?;
             // log::info!("Positions - {:?}", res);
             // let pos = res
             //     .into_iter()
             //     .filter(|v| v.position_id == order_res.order_report.pos_main_rept_id)
             //     .next()
             //     .unwrap();
             //
             // log::info!("Close position");
             // let res = client.close_position(pos).await?;
             // log::info!("Result of close position - {:?}", res);
        }

        { // Test : stop order
             // log::info!("New stop order");
             // let res = client
             //     .new_stop_order(1, Side::BUY, 1.1, 1000.0, None, None, None)
             //     .await?;
             // log::info!("Result of stop order - {:?}", res);
        }

        { // Test : limit order & replace price & cancel order
             // log::info!("New limit order");
             // let res = client
             //     .new_limit_order(1, Side::BUY, 0.8, 1000.0, None, None, None)
             //     .await?;
             // log::info!("Result of limit order - {:?}", res);
             // async_std::task::sleep(std::time::Duration::from_secs(2)).await;
             //
             // log::info!("Replace the order");
             // let res = client
             //     .replace_order(
             //         Some(res.order_report.cl_ord_id),
             //         None,
             //         res.order_report.order_qty,
             //         Some(0.82),
             //         None,
             //         None,
             //     )
             //     .await?;
             //
             // log::info!("Result of replace order - {:?}", res);
             // async_std::task::sleep(std::time::Duration::from_secs(2)).await;
             //
             // log::info!("Cancel the order");
             //
             // // !!both works
             // let res = client
             //     .cancel_order(Some(res.order_report.order_id), None)
             //     // .cancel_order(None, Some(res.order_report.cl_ord_id))
             //     .await;
             // log::info!("Result of cancel order - {:?}", res);
        }

        // for test
        // async_std::task::sleep(std::time::Duration::from_secs(400)).await;
        async_std::task::sleep(std::time::Duration::from_secs(2)).await;
    }

    // disconnect
    client.disconnect().await?;
    async_std::task::sleep(std::time::Duration::from_secs(2)).await;

    Ok(())
}
