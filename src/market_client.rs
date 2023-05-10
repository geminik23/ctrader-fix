use std::collections::HashMap;

use crate::{
    fixapi::FixApi,
    messages::MarketDataReq,
    types::{ConnectionHandler, Error},
};

pub struct MarketClient {
    internal: FixApi,
    req_symbols: HashMap<u32, String>,
    // subscribed_symbols: HashMap<String, String>,
}

impl MarketClient {
    pub fn new(
        host: String,
        login: String,
        password: String,
        broker: String,
        heartbeat_interval: Option<u32>,
    ) -> Self {
        Self {
            internal: FixApi::new(
                crate::types::SubID::QUOTE,
                host,
                login,
                password,
                broker,
                heartbeat_interval,
            ),
            req_symbols: HashMap::new(),
            // subscribed_symbols: HashMap::new(),
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
        self.internal.disconnect().await
    }

    pub fn is_connected(&self) -> bool {
        self.internal.is_connected()
    }

    pub async fn subscribe(&mut self, symbol_id: u32) -> Result<(), Error> {
        // symbol and
        // intialize the request
        let req = MarketDataReq::new("-1".into(), '1', 1, None, 2, '0', 1, symbol_id);
        self.internal.send_message(req).await?;

        // wait
        Ok(())
    }
}
