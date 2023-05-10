use std::collections::{HashMap, HashSet};

use crate::{
    fixapi::FixApi,
    messages::MarketDataReq,
    types::{ConnectionHandler, Error, Field},
};

pub struct MarketClient {
    internal: FixApi,

    requested_symbol: HashSet<u32>,
    market_data: HashMap<u32, (f64, f64)>,
}

impl MarketClient {
    pub fn new(
        host: String,
        login: String,
        password: String,
        broker: String,
        heartbeat_interval: Option<u32>,
    ) -> Self {
        let mut internal = FixApi::new(
            crate::types::SubID::QUOTE,
            host,
            login,
            password,
            broker,
            heartbeat_interval,
        );

        let trigger = internal.trigger.clone();
        let market_callback = move |s: String| {
            // TODO
            // what todo?
            let tx = trigger.clone();
            async move {
                // TODO
                tx.send(()).await.unwrap_or_else(|e| {
                    // fatal
                    log::error!("Failed to notify that the response is received - {:?}", e);
                });
                ()
            }
        };
        internal.register_market_callback(market_callback);

        Self {
            internal,
            requested_symbol: HashSet::new(),
            market_data: HashMap::new(),
        }
    }

    pub fn register_connection_handler<T: ConnectionHandler + Send + Sync + 'static>(
        &mut self,
        handler: T,
    ) {
        self.internal.register_connection_handler(handler);
    }

    pub async fn connect(&mut self) -> Result<(), Error> {
        self.internal.connect().await?;
        self.internal.logon().await
    }

    pub async fn disconnect(&mut self) -> Result<(), Error> {
        // self.internal.logout().await?;
        self.internal.disconnect().await
    }

    pub fn is_connected(&self) -> bool {
        self.internal.is_connected()
    }

    pub async fn subscribe(&mut self, symbol_id: u32) -> Result<(), Error> {
        // check the req and market_data
        if self.requested_symbol.is_empty() && self.market_data.is_empty() {
            // TODO
            // register the closures.
        }

        // add to requested symbol.
        self.requested_symbol.insert(symbol_id);

        // intialize the request and send req
        let req = MarketDataReq::new("-1".into(), '1', 1, None, &['0', '1'], 1, symbol_id);
        let seq_num = self.internal.send_message(req).await?;

        // TODO
        // wait the rx
        self.internal.wait_notifier().await?;

        {
            let mut cont = self.internal.container.write().await;
            //check the seq no
            if let Some(res) = cont.remove(&seq_num) {
                if res.get_message_type() == "3" {
                    log::error!("Failed to subscribe the symbol_id {:?}", symbol_id);
                    self.requested_symbol.remove(&symbol_id);
                    return Err(Error::SubscriptionError(
                        symbol_id,
                        res.get_field_value(Field::Text)
                            .expect("No Text tag in Reject response"),
                    ));
                }
            }
        }

        // rejected check with
        // success? -> "W" and check the Symbol (symbol_id)

        self.requested_symbol.remove(&symbol_id);
        //

        Ok(())
    }

    pub async fn unsubscribe(&mut self, symbol_id: u32) -> Result<(), Error> {
        // TODO
        // let req = MarketDataReq::new("-1".into(), '1', 1, None, &['0', '1'], 1, symbol_id);
        // let seq_num = self.internal.send_message(req).await?;

        // wait the rx
        self.internal.wait_notifier().await?;

        // remove the symbol
        self.market_data.remove(&symbol_id);

        Ok(())
    }
}
