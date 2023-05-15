use chrono::{NaiveDateTime, Utc};
use uuid::Uuid;

use crate::{
    fixapi::FixApi,
    messages::{
        NewOrderSingleReq, OrderMassStatusReq, PositionsReq, ResponseMessage, SecurityListReq,
    },
    parse_func,
    types::{
        ConnectionHandler, Error, Field, NewOrderReport, OrderStatusReport, OrderType,
        PositionReport, Side, SymbolInformation, DELIMITER,
    },
};
use std::{collections::HashMap, sync::Arc};

pub struct TradeClient {
    internal: FixApi,
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

    async fn fetch_response(
        &self,
        arg: Vec<(&str, Field, String)>,
    ) -> Result<Vec<ResponseMessage>, Error> {
        let arg = arg.into_iter().map(|v| (v.0, v)).collect::<HashMap<_, _>>();
        while let Ok(msg_type) = self.internal.wait_notifier().await {
            let has_key = arg.contains_key(&msg_type.as_str());
            if has_key {
                match self.internal.check_responses(arg.clone()).await {
                    Ok(res) => {
                        log::debug!("in fetch response - {:?}", res);
                        return Ok(res);
                    }
                    Err(Error::NoResponse) => {
                        // log::debug!("no reponse {:?}", msg_type);
                        if let Err(err) = self.internal.trigger.send(msg_type).await {
                            return Err(Error::TriggerError(err));
                        }
                    }
                    Err(err) => {
                        log::debug!("err in fetch response for {} - {:?}", msg_type, err);
                        return Err(err);
                    }
                }
            } else {
                if let Err(err) = self.internal.trigger.send(msg_type).await {
                    return Err(Error::TriggerError(err));
                }
            }
        }
        Err(Error::UnknownError)
    }

    fn create_unique_id(&self) -> String {
        Uuid::new_v4().to_string()
    }

    /// Fetch the security list from the server.
    ///
    ///
    /// This is asn asynchronous method that sends a request to the server and waits for the
    /// response. It returns a result containing the data if the request succesful, or an error if
    /// it fails.
    pub async fn fetch_security_list(&self) -> Result<Vec<SymbolInformation>, Error> {
        let security_req_id = self.create_unique_id();
        let req = SecurityListReq::new(security_req_id.clone(), 0, None);
        self.internal.send_message(req).await?;
        match self
            .fetch_response(vec![("y", Field::SecurityReqID, security_req_id)])
            .await
        {
            Ok(res) => {
                let res = res.first().unwrap();
                parse_func::parse_security_list(res)
            }
            Err(err) => Err(err),
        }
    }

    pub async fn fetch_positions(&self) -> Result<Vec<PositionReport>, Error> {
        let pos_req_id = self.create_unique_id();
        let req = PositionsReq::new(pos_req_id.clone(), None);
        self.internal.send_message(req).await?;

        match self
            .fetch_response(vec![("AP", Field::PosReqID, pos_req_id)])
            .await
        {
            Ok(res) => {
                let res = res.first().unwrap();
                parse_func::parse_positions(res)
            }
            Err(err) => Err(err),
        }
    }

    pub async fn fetch_all_order_status(
        &self,
        issue_data: Option<NaiveDateTime>,
    ) -> Result<Vec<OrderStatusReport>, Error> {
        let mass_status_req_id = self.create_unique_id();
        // FIXME if mass_status_req_id is not 7, then return 'j' but response does not include the mass_status_req_id
        let req = OrderMassStatusReq::new(mass_status_req_id.clone(), 7, issue_data);
        self.internal.send_message(req).await?;

        match self
            .fetch_response(vec![
                ("8", Field::MassStatusReqID, mass_status_req_id.clone()),
                ("j", Field::BusinessRejectRefID, mass_status_req_id.clone()),
            ])
            .await
        {
            Ok(res) => {
                if let Some(_) = res
                    .iter()
                    .filter(|r| r.get_field_value(Field::MsgType).unwrap() == "j")
                    .next()
                {
                    // not error: order not found
                    return Ok(Vec::new());
                    // let reason = rej
                    //     .get_field_value(Field::Text)
                    //     .unwrap_or("Rejected".into());
                    // return Err(Error::RequestRejected(reason));
                }

                // FIXME unnecessary line
                if let Some(res) = res
                    .into_iter()
                    .filter(|r| r.get_field_value(Field::MsgType).unwrap() == "8")
                    .next()
                {
                    return parse_func::parse_order_status(res);
                }

                // let res = res.first().unwrap();
                // parse_positions(res)
                //
                Err(Error::UnknownError)
            }
            Err(err) => Err(err),
        }
        // let res = self.fetch_response(seq_num).await?;
        // parse_order_mass(res)
    }

    async fn new_order(&self, req: NewOrderSingleReq) -> Result<Vec<ResponseMessage>, Error> {
        let cl_ord_id = req.cl_ord_id.clone();

        self.internal.send_message(req).await?;
        self.fetch_response(vec![
            ("8", Field::ClOrdId, cl_ord_id.clone()),
            ("j", Field::BusinessRejectRefID, cl_ord_id.clone()),
        ])
        .await
    }

    pub async fn new_market_order(
        &self,
        symbol: u32,
        side: Side,
        order_qty: f64,
        cl_ord_id: Option<String>,
        pos_id: Option<String>,
        transact_time: Option<NaiveDateTime>,
        custom_ord_label: Option<String>,
    ) -> Result<NewOrderReport, Error> {
        let req = NewOrderSingleReq::new(
            cl_ord_id.unwrap_or(self.create_unique_id()),
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

        match self.new_order(req).await {
            Ok(res) => {
                if let Some(rej) = res
                    .iter()
                    .filter(|r| r.get_field_value(Field::MsgType).unwrap() == "j")
                    .next()
                {
                    // Order Rejected
                    return Err(Error::OrderRejected(
                        rej.get_field_value(Field::Text).unwrap_or("Unknown".into()),
                    ));
                }

                if let Some(res) = res
                    .into_iter()
                    .filter(|r| r.get_field_value(Field::MsgType).unwrap() == "8")
                    .next()
                {
                    return parse_func::parse_new_order_report(res);
                }
                //
                Err(Error::UnknownError)
            }
            Err(err) => Err(err),
        }
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
