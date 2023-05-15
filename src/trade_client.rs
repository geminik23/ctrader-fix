use chrono::{NaiveDateTime, Utc};

use crate::{
    fixapi::FixApi,
    messages::{
        NewOrderSingleReq, OrderMassStatusReq, PositionsReq, ResponseMessage, SecurityListReq,
    },
    types::{
        ConnectionHandler, Error, ExecutionReport, Field, OrderType, PositionReport, Side,
        SymbolInformation,
    },
};
use std::sync::Arc;

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

fn parse_positions(res: ResponseMessage) -> Result<Vec<PositionReport>, Error> {
    println!("{:?}", res);
    let pos_list = res.get_repeating_groups(Field::TotalNumPosReports, Field::PosReqResult, None);
    let mut result = Vec::new();
    for pos in pos_list.into_iter() {
        let pos_req_result = pos.get(&Field::PosReqResult).unwrap();
        // TODO
        // println!("{:?}", pos);
        if pos_req_result == "2" {
            continue;
        }
    }

    Ok(result)
}

fn parse_order_mass(res: ResponseMessage) -> Result<Vec<ExecutionReport>, Error> {
    let mut result = Vec::new();
    // TODO

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
                crate::types::SubID::TRADE,
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
        while let Ok(msg_type) = self.internal.wait_notifier().await {
            match self.internal.check_req_accepted(seq_num).await {
                Ok(res) => {
                    log::debug!("in res {:?}", seq_num);
                    return Ok(res);
                }
                Err(Error::NoResponse(_)) => {
                    log::debug!(" no reponse{:?}", seq_num);
                    if let Err(err) = self.internal.trigger.send(msg_type).await {
                        return Err(Error::TriggerError(err));
                    }
                }
                Err(err) => {
                    log::debug!("err {:?}", seq_num);
                    return Err(err);
                }
            }
        }
        Err(Error::NoResponse(seq_num))
    }

    /// Fetch the security list from the server.
    ///
    ///
    /// This is asn asynchronous method that sends a request to the server and waits for the
    /// response. It returns a result containing the data if the request succesful, or an error if
    /// it fails.
    pub async fn fetch_security_list(&self) -> Result<Vec<SymbolInformation>, Error> {
        let req = SecurityListReq::new("1".into(), 0, None);
        let seq_num = self.internal.send_message(req).await?;
        println!("{:?}", seq_num);
        parse_security_list(self.fetch_response(seq_num).await?)
    }

    pub async fn fetch_positions(&self) -> Result<Vec<PositionReport>, Error> {
        let req = PositionsReq::new(uuid::Uuid::new_v4().to_string(), None);
        let seq_num = self.internal.send_message(req).await?;
        let res = self.fetch_response(seq_num).await?;
        parse_positions(res)
    }

    pub async fn fetch_all_orders(&self) -> Result<Vec<ExecutionReport>, Error> {
        let req = OrderMassStatusReq::new(uuid::Uuid::new_v4().to_string(), 7, None);
        let seq_num = self.internal.send_message(req).await?;
        let res = self.fetch_response(seq_num).await?;
        parse_order_mass(res)
    }

    pub async fn new_market_order(
        &self,
        symbol: u32,
        side: Side,
        order_qty: f64,
        cl_orig_id: Option<String>,
        pos_id: Option<String>,
        transact_time: Option<NaiveDateTime>,
        custom_ord_label: Option<String>,
    ) -> Result<(), Error> {
        let req = NewOrderSingleReq::new(
            cl_orig_id.unwrap_or(format!("dt{:?}", Utc::now())),
            symbol,
            side,
            transact_time,
            order_qty,
            OrderType::MARKET,
            None,
            None,
            None,
            pos_id,
            custom_ord_label,
        );
        let seq_num = self.internal.send_message(req).await?;
        let res = self.fetch_response(seq_num).await?;
        println!("{:?}", res);

        // TODO handle response
        Ok(())
    }

    pub async fn new_limit_order(&self) -> Result<(), Error> {
        unimplemented!()
    }

    pub async fn new_stop_order(&self) -> Result<(), Error> {
        unimplemented!()
    }

    pub async fn replace_order(&self) -> Result<(), Error> {
        unimplemented!()
    }

    pub async fn close_position(&self) -> Result<(), Error> {
        unimplemented!()
    }

    pub async fn close_all_position(&self) -> Result<(), Error> {
        unimplemented!()
    }

    pub async fn cancel_order(&self) -> Result<(), Error> {
        unimplemented!()
    }

    pub async fn cancel_all_position(&self) -> Result<(), Error> {
        unimplemented!()
    }
}
