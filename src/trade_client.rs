use std::sync::Arc;

use crate::{
    fixapi::FixApi,
    messages::{ResponseMessage, SecurityListReq},
    types::{ConnectionHandler, Error, Field, SymbolInformation},
};

pub struct TradeClient {
    internal: FixApi,
}

fn parse_security_list(res: ResponseMessage) -> Result<Vec<SymbolInformation>, Error> {
    let sec_list = res.get_repeating_groups(Field::NoRelatedSym, Field::Symbol, None);
    let mut result = Vec::new();
    for symbol in sec_list.into_iter() {
        if symbol.len() < 3 {
            continue;
        }
        result.push(SymbolInformation {
            name: symbol
                .get(&Field::SymbolName)
                .ok_or(Error::FieldNotFoundError(Field::SymbolName))?
                .clone(),
            id: symbol
                .get(&Field::Symbol)
                .ok_or(Error::FieldNotFoundError(Field::Symbol))?
                .parse::<u32>()
                .unwrap(),
            digits: symbol
                .get(&Field::SymbolDigits)
                .ok_or(Error::FieldNotFoundError(Field::SymbolDigits))?
                .parse::<u32>()
                .unwrap(),
        });
    }
    Ok(result)
}

impl TradeClient {
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
        }
    }

    pub fn register_connection_handler<T: ConnectionHandler + Send + Sync + 'static>(
        &mut self,
        handler: T,
    ) {
        self.internal.register_connection_handler(handler);
    }

    pub fn register_connection_handler_arc<T: ConnectionHandler + Send + Sync + 'static>(
        &mut self,
        handler: Arc<T>,
    ) {
        self.internal.register_connection_handler_arc(handler);
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

    async fn fetch_response(&self, seq_num: u32) -> Result<ResponseMessage, Error> {
        self.internal.wait_notifier().await?;
        while let Ok(()) = self.internal.wait_notifier().await {
            match self.internal.check_req_accepted(seq_num).await {
                Ok(res) => {
                    return Ok(res);
                }
                Err(Error::NoResponse(_)) => {
                    if let Err(err) = self.internal.trigger.send(()).await {
                        return Err(Error::TriggerError(err));
                    }
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
        Err(Error::NoResponse(seq_num))
    }

    /// Fetch the security list from the server.
    ///
    /// **NOTE** : It tooks a long time to fetch data**
    ///
    /// This is asn asynchronous method that sends a request to the server and waits for the
    /// response. It returns a result containing the data if the request succesful, or an error if
    /// it fails.
    pub async fn fetch_security_list(&self) -> Result<Vec<SymbolInformation>, Error> {
        // FIXME use the callback
        let req = SecurityListReq::new("1".into(), 0, None);
        let seq_num = self.internal.send_message(req).await?;
        parse_security_list(self.fetch_response(seq_num).await?)
    }
}
