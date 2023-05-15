use chrono::{NaiveDateTime, Utc};
use uuid::Uuid;

use crate::{
    fixapi::FixApi,
    messages::{
        NewOrderSingleReq, OrderMassStatusReq, PositionsReq, ResponseMessage, SecurityListReq,
    },
    types::{
        ConnectionHandler, Error, ExecutionReport, Field, OrderType, PositionReport, Side,
        SymbolInformation, DELIMITER,
    },
};
use std::sync::Arc;

pub struct TradeClient {
    internal: FixApi,
}

fn parse_security_list(res: &ResponseMessage) -> Result<Vec<SymbolInformation>, Error> {
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

fn parse_positions(res: &ResponseMessage) -> Result<Vec<PositionReport>, Error> {
    let npos = res
        .get_field_value(Field::TotalNumPosReports)
        .unwrap_or("0".into())
        .parse::<u32>()
        .unwrap_or(0);

    let mut raw_res: Vec<ResponseMessage> = Vec::new();
    if npos > 1 {
        let parts: Vec<_> = res.get_message().split("|80=").collect();
        let first = parts[0];
        raw_res.push(ResponseMessage::new(&format!("{}|", first), DELIMITER));
        let parts: Vec<_> = parts
            .iter()
            .skip(1)
            .map(|part| ResponseMessage::new(&format!("80={}|", part), DELIMITER))
            .collect();
        raw_res.extend(parts);
    } else {
        raw_res.push(ResponseMessage::new(res.get_message(), DELIMITER));
    }

    Ok(raw_res
        .into_iter()
        .filter(|res| res.get_field_value(Field::PosReqResult).unwrap() == "0")
        .filter(|res| {
            res.get_field_value(Field::NoPositions)
                .map(|v| v == "1")
                .unwrap_or(false)
        })
        .map(|res| PositionReport {
            symbol_id: res
                .get_field_value(Field::Symbol)
                .unwrap()
                .parse::<u32>()
                .unwrap(),
            position_id: res.get_field_value(Field::PosMaintRptID).unwrap(),
            long_qty: res
                .get_field_value(Field::LongQty)
                .unwrap()
                .parse::<f64>()
                .unwrap(),
            short_qty: res
                .get_field_value(Field::ShortQty)
                .unwrap()
                .parse::<f64>()
                .unwrap(),
            settle_price: res
                .get_field_value(Field::SettlPrice)
                .unwrap()
                .parse::<f64>()
                .unwrap(),
            absolute_tp: res
                .get_field_value(Field::AbsoluteTP)
                .map(|v| v.parse::<f64>().unwrap()),
            absolute_sl: res
                .get_field_value(Field::AbsoluteSL)
                .map(|v| v.parse::<f64>().unwrap()),
            trailing_sl: res.get_field_value(Field::TrailingSL).map(|v| v == "Y"),
            trigger_method_sl: res
                .get_field_value(Field::TriggerMethodSL)
                .map(|v| v.parse::<u32>().unwrap()),
            guaranteed_sl: res.get_field_value(Field::GuaranteedSL).map(|v| v == "Y"),
        })
        .collect())
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

    async fn fetch_response(
        &self,
        types: Vec<&str>,
        field: Field,
        value: String,
    ) -> Result<Vec<ResponseMessage>, Error> {
        while let Ok(msg_type) = self.internal.wait_notifier().await {
            if types.contains(&msg_type.as_str()) {
                match self
                    .internal
                    .check_responses(&msg_type, field, value.clone())
                    .await
                {
                    Ok(res) => {
                        log::debug!("in fetch response - {:?}", res);
                        return Ok(res);
                    }
                    Err(Error::NoResponse(msg_type)) => {
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

    /// Fetch the security list from the server.
    ///
    ///
    /// This is asn asynchronous method that sends a request to the server and waits for the
    /// response. It returns a result containing the data if the request succesful, or an error if
    /// it fails.
    pub async fn fetch_security_list(&self) -> Result<Vec<SymbolInformation>, Error> {
        let security_req_id = Uuid::new_v4().to_string();
        let req = SecurityListReq::new(security_req_id.clone(), 0, None);
        self.internal.send_message(req).await?;
        match self
            .fetch_response(vec!["y"], Field::SecurityReqID, security_req_id)
            .await
        {
            Ok(res) => {
                let res = res.first().unwrap();
                parse_security_list(res)
            }
            Err(err) => Err(err),
        }
    }

    pub async fn fetch_positions(&self) -> Result<Vec<PositionReport>, Error> {
        let pos_req_id = Uuid::new_v4().to_string();
        let req = PositionsReq::new(pos_req_id.clone(), None);
        self.internal.send_message(req).await?;

        match self
            .fetch_response(vec!["AP"], Field::PosReqID, pos_req_id)
            .await
        {
            Ok(res) => {
                let res = res.first().unwrap();
                parse_positions(res)
            }
            Err(err) => Err(err),
        }
    }
    //
    // pub async fn fetch_all_orders(&self) -> Result<Vec<ExecutionReport>, Error> {
    //     let req = OrderMassStatusReq::new(uuid::Uuid::new_v4().to_string(), 7, None);
    //     let seq_num = self.internal.send_message(req).await?;
    //     let res = self.fetch_response(seq_num).await?;
    //     parse_order_mass(res)
    // }

    // pub async fn new_market_order(
    //     &self,
    //     symbol: u32,
    //     side: Side,
    //     order_qty: f64,
    //     cl_orig_id: Option<String>,
    //     pos_id: Option<String>,
    //     transact_time: Option<NaiveDateTime>,
    //     custom_ord_label: Option<String>,
    // ) -> Result<(), Error> {
    //     let req = NewOrderSingleReq::new(
    //         cl_orig_id.unwrap_or(format!("dt{:?}", Utc::now())),
    //         symbol,
    //         side,
    //         transact_time,
    //         order_qty,
    //         OrderType::MARKET,
    //         None,
    //         None,
    //         None,
    //         pos_id,
    //         custom_ord_label,
    //     );
    //     let seq_num = self.internal.send_message(req).await?;
    //     let res = self.fetch_response(seq_num).await?;
    //     println!("{:?}", res);
    //
    //     // TODO handle response
    //     Ok(())
    // }

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
