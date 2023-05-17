use async_std::{
    channel::{bounded, Receiver, Sender},
    sync::RwLock,
    task,
};
use chrono::NaiveDateTime;
use uuid::Uuid;

use crate::{
    fixapi::FixApi,
    messages::{
        NewOrderSingleReq, OrderCancelReplaceReq, OrderCancelReq, OrderMassStatusReq, PositionsReq,
        ResponseMessage, SecurityListReq,
    },
    parse_func::{self, parse_execution_report},
    types::{
        ConnectionHandler, Error, ExecutionReport, Field, OrderType, PositionReport, Side,
        SymbolInformation, TradeDataHandler,
    },
};

use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

#[derive(Debug)]
struct TimeoutItem<T> {
    item: T,
    expiry: Instant,
    read: AtomicBool,
}

impl<T> TimeoutItem<T> {
    fn new(item: T, lifetime: Duration) -> Self {
        TimeoutItem {
            item,
            expiry: Instant::now() + lifetime,
            read: AtomicBool::new(false),
        }
    }
}

pub struct TradeClient {
    internal: FixApi,

    trade_data_handler: Option<Arc<dyn TradeDataHandler + Send + Sync>>,

    queue: Arc<RwLock<VecDeque<TimeoutItem<ResponseMessage>>>>,

    signal: Sender<()>,
    receiver: Receiver<()>,

    // for waiting response in fetch methods.
    timeout: u64,
}

impl TradeClient {
    pub fn new(
        host: String,
        login: String,
        password: String,
        sender_comp_id: String,
        heartbeat_interval: Option<u32>,
    ) -> Self {
        let (tx, rx) = bounded(1);
        Self {
            internal: FixApi::new(
                crate::types::SubID::TRADE,
                host,
                login,
                password,
                sender_comp_id,
                heartbeat_interval,
            ),
            trade_data_handler: None,
            queue: Arc::new(RwLock::new(VecDeque::new())),

            signal: tx,
            receiver: rx,

            timeout: 5000, //
        }
    }

    pub fn register_trade_handler_arc<T: TradeDataHandler + Send + Sync + 'static>(
        &mut self,
        handler: Arc<T>,
    ) {
        self.trade_data_handler = Some(handler);
    }

    pub fn register_trade_handler<T: TradeDataHandler + Send + Sync + 'static>(
        &mut self,
        handler: T,
    ) {
        self.trade_data_handler = Some(Arc::new(handler));
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
        self.register_internal_handler();
        self.internal.connect().await?;
        self.internal.logon().await
    }

    pub async fn disconnect(&mut self) -> Result<(), Error> {
        self.internal.disconnect().await
    }

    pub fn is_connected(&self) -> bool {
        self.internal.is_connected()
    }

    fn register_internal_handler(&mut self) {
        let queue = self.queue.clone();
        let handler = self.trade_data_handler.clone();
        let signal = self.signal.clone();
        let trade_callback = move |res: ResponseMessage| {
            let signal = signal.clone();
            let handler = handler.clone();
            let queue = queue.clone();
            let lifetime = Duration::from_millis(5000);
            task::spawn(async move {
                match res.get_message_type() {
                    "8" => {
                        if res
                            .get_field_value(Field::ExecType)
                            .map(|v| v.as_str() != "I")
                            .unwrap_or(true)
                        {
                            match parse_execution_report(res.clone()) {
                                Ok(report) => {
                                    if let Some(handler) = handler {
                                        handler.on_execution_report(report).await;
                                    }
                                }
                                Err(_err) => {
                                    // IGNORE
                                }
                            }
                        }
                    }
                    _ => {}
                }

                queue
                    .write()
                    .await
                    .push_back(TimeoutItem::new(res, lifetime));

                // check timeout
                let now = Instant::now();
                loop {
                    let expiry = queue.read().await.front().map(|v| v.expiry).unwrap_or(now);
                    if expiry < now {
                        // pop old item
                        queue.write().await.pop_front();
                    } else {
                        break;
                    }
                }

                signal.try_send(()).ok();
                // signal.send(()).await.ok();
            });
        };

        self.internal.register_trade_callback(trade_callback);
    }

    fn create_unique_id(&self) -> String {
        Uuid::new_v4().to_string()
    }

    async fn wait_notifier(&self, receiver: Receiver<()>, dur: u64) -> Result<(), Error> {
        if !self.is_connected() {
            return Err(Error::NotConnected);
        }
        async_std::future::timeout(Duration::from_millis(dur), receiver.recv())
            .await
            .map_err(|_| Error::TimeoutError)?
            .map_err(|e| e.into())
    }

    async fn fetch_response(
        &self,
        arg: Vec<(&str, Field, String)>,
    ) -> Result<ResponseMessage, Error> {
        // setup for timeout
        let now = Instant::now();
        let mut remain = self.timeout;

        loop {
            let _ = self.wait_notifier(self.receiver.clone(), remain).await?;
            // match self.wait_notifier(receiver, remain).await {
            let mut res = None;
            let q = self.queue.read().await;
            for v in q.iter().rev() {
                let mut b = false;
                let read = v.read.load(Ordering::Relaxed);
                if read {
                    continue;
                }

                for (msg_type, field, value) in arg.iter() {
                    if v.item.matching_field_value(msg_type, *field, value) {
                        b = true;
                        res = Some(v.item.clone());
                        v.read.store(true, Ordering::Relaxed);
                        break;
                    }
                }
                if b {
                    break;
                }
            }

            match res {
                Some(res) => {
                    return Ok(res);
                }
                None => {
                    // check remaining time.
                    let past = (Instant::now() - now).as_millis() as u64;
                    if past < self.timeout {
                        // continue.
                        remain = self.timeout - past;

                        // check if there is more waiting receiver.
                        // FIXME
                        if self.receiver.receiver_count() > 1 {
                            self.signal.try_send(()).ok();
                        }
                        continue;
                    } else {
                        return Err(Error::TimeoutError);
                    }
                }
            }
        }
    }

    fn check_connection(&self) -> Result<(), Error> {
        if self.is_connected() {
            Ok(())
        } else {
            Err(Error::NotConnected)
        }
    }

    /// Fetch the security list from the server.
    ///
    ///
    /// This is asn asynchronous method that sends a request to the server and waits for the
    /// response. It returns a result containing the data if the request succesful, or an error if
    /// it fails.
    pub async fn fetch_security_list(&self) -> Result<Vec<SymbolInformation>, Error> {
        self.check_connection()?;
        let security_req_id = self.create_unique_id();
        let req = SecurityListReq::new(security_req_id.clone(), 0, None);
        self.internal.send_message(req).await?;
        match self
            .fetch_response(vec![("y", Field::SecurityReqID, security_req_id)])
            .await
        {
            Ok(res) => parse_func::parse_security_list(&res),
            Err(err) => Err(err),
        }
    }

    pub async fn fetch_positions(&self) -> Result<Vec<PositionReport>, Error> {
        self.check_connection()?;
        let pos_req_id = self.create_unique_id();
        let req = PositionsReq::new(pos_req_id.clone(), None);
        self.internal.send_message(req).await?;

        let mut result = Vec::new();

        loop {
            match self
                .fetch_response(vec![("AP", Field::PosReqID, pos_req_id.clone())])
                .await
            {
                Ok(res) => {
                    if res.get_message_type() == "AP"
                        && res
                            .get_field_value(Field::PosReqResult)
                            .map_or(false, |v| v.as_str() == "0")
                    {
                        let no_pos = res
                            .get_field_value(Field::TotalNumPosReports)
                            .unwrap_or("0".into())
                            .parse::<usize>()
                            .unwrap();
                        result.push(res);
                        if no_pos <= result.len() {
                            return parse_func::parse_positions(result);
                        } else {
                            continue;
                        }
                    } else {
                        return parse_func::parse_positions(vec![res]);
                    }
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
    }

    pub async fn fetch_all_order_status(
        &self,
        issue_data: Option<NaiveDateTime>,
    ) -> Result<Vec<ExecutionReport>, Error> {
        self.check_connection()?;
        let mass_status_req_id = self.create_unique_id();
        // FIXME if mass_status_req_id is not 7, then return 'j' but response does not include the mass_status_req_id
        let req = OrderMassStatusReq::new(mass_status_req_id.clone(), 7, issue_data);
        self.internal.send_message(req).await?;

        let mut result = Vec::new();

        loop {
            match self
                .fetch_response(vec![
                    ("8", Field::MassStatusReqID, mass_status_req_id.clone()),
                    ("j", Field::BusinessRejectRefID, mass_status_req_id.clone()),
                ])
                .await
            {
                Ok(res) => {
                    return match res.get_message_type() {
                        "j" => Ok(Vec::new()),
                        "8" => {
                            let no_report = res
                                .get_field_value(Field::TotNumReports)
                                .unwrap_or("0".into())
                                .parse::<usize>()
                                .unwrap();

                            result.push(res);

                            if no_report <= result.len() {
                                parse_func::parse_order_mass_status(result)
                            } else {
                                continue;
                            }
                        }
                        _ => Err(Error::UnknownError),
                    };
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
        // let res = self.fetch_response(seq_num).await?;
        // parse_order_mass(res)
    }

    async fn new_order(&self, req: NewOrderSingleReq) -> Result<ExecutionReport, Error> {
        self.check_connection()?;
        let cl_ord_id = req.cl_ord_id.clone();

        self.internal.send_message(req).await?;
        match self
            .fetch_response(vec![
                ("8", Field::ClOrdId, cl_ord_id.clone()),
                ("j", Field::BusinessRejectRefID, cl_ord_id.clone()),
            ])
            .await
        {
            Ok(res) => match res.get_message_type() {
                "j" => Err(Error::OrderFailed(
                    res.get_field_value(Field::Text).unwrap_or("Unknown".into()),
                )),
                "8" => parse_func::parse_execution_report(res),
                _ => Err(Error::UnknownError),
            },
            Err(err) => Err(err),
        }
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
    ) -> Result<ExecutionReport, Error> {
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
        self.new_order(req).await
    }

    pub async fn new_limit_order(
        &self,
        symbol: u32,
        side: Side,
        price: f64,
        order_qty: f64,
        cl_ord_id: Option<String>,
        pos_id: Option<String>,
        expire_time: Option<NaiveDateTime>,
        transact_time: Option<NaiveDateTime>,
        custom_ord_label: Option<String>,
    ) -> Result<ExecutionReport, Error> {
        let req = NewOrderSingleReq::new(
            cl_ord_id.unwrap_or(self.create_unique_id()),
            symbol,
            side,
            transact_time,
            order_qty,
            OrderType::LIMIT,
            Some(price),
            None,
            expire_time,
            pos_id,
            custom_ord_label,
        );

        self.new_order(req).await
    }

    pub async fn new_stop_order(
        &self,
        symbol: u32,
        side: Side,
        stop_px: f64,
        order_qty: f64,
        cl_ord_id: Option<String>,
        pos_id: Option<String>,
        expire_time: Option<NaiveDateTime>,
        transact_time: Option<NaiveDateTime>,
        custom_ord_label: Option<String>,
    ) -> Result<ExecutionReport, Error> {
        let req = NewOrderSingleReq::new(
            cl_ord_id.unwrap_or(self.create_unique_id()),
            symbol,
            side,
            transact_time,
            order_qty,
            OrderType::STOP,
            None,
            Some(stop_px),
            expire_time,
            pos_id,
            custom_ord_label,
        );

        self.new_order(req).await
    }
    pub async fn close_position(
        &self,
        pos_report: PositionReport,
    ) -> Result<ExecutionReport, Error> {
        self.adjust_position_size(
            pos_report.position_id,
            pos_report.symbol_id,
            if pos_report.long_qty == 0.0 {
                pos_report.short_qty
            } else {
                pos_report.long_qty
            },
            if pos_report.long_qty == 0.0 {
                Side::BUY
            } else {
                Side::SELL
            },
        )
        .await
    }

    /// Adjusts the size of a position.
    ///
    /// This method takes a position id, symbol_id, a side (buy or sell), and a lot size.
    /// If the position exists, it adjusts the size of the position by adding or subtracting the given lot size.
    /// If the side is 'buy', the lot size is added to the position.
    /// If the side is 'sell', the lot size is subtracted from the position.
    pub async fn adjust_position_size(
        &self,
        pos_id: String,
        symbol_id: u32,
        lot: f64,
        side: Side,
    ) -> Result<ExecutionReport, Error> {
        let req = NewOrderSingleReq::new(
            self.create_unique_id(),
            symbol_id,
            side,
            None,
            lot,
            OrderType::MARKET,
            None,
            None,
            None,
            Some(pos_id),
            None,
        );

        self.new_order(req).await
    }

    /// Replace order request
    ///
    /// # Arguments
    ///
    /// * `orig_cl_ord_id` - A unique identifier for the order, which is going to be canceled, allocated by the client.
    /// * `order_id` - Unique ID of an order, returned by the server.
    /// ...
    ///
    ///  Either `orig_cl_ord_id` or `order_id` must be passed to this function. If both are `None`, the function will return an error.
    pub async fn replace_order(
        &self,
        org_cl_ord_id: Option<String>,
        order_id: Option<String>,
        order_qty: f64,
        price: Option<f64>,
        stop_px: Option<f64>,
        expire_time: Option<NaiveDateTime>,
    ) -> Result<ExecutionReport, Error> {
        if org_cl_ord_id.is_none() && order_id.is_none() {
            return Err(Error::MissingArgumentError);
        }
        self.check_connection()?;
        let orgid = match org_cl_ord_id.clone() {
            Some(v) => v,
            None => order_id.clone().unwrap(),
        };
        let oid = match order_id.clone() {
            Some(v) => v,
            None => org_cl_ord_id.clone().unwrap(),
        };
        let cl_ord_id = self.create_unique_id();
        let req = OrderCancelReplaceReq::new(
            orgid,
            Some(oid),
            cl_ord_id.clone(),
            order_qty,
            price,
            stop_px,
            expire_time,
        );
        self.internal.send_message(req).await?;
        match self
            .fetch_response(vec![
                if org_cl_ord_id.is_some() {
                    ("8", Field::ClOrdId, org_cl_ord_id.unwrap())
                } else {
                    ("8", Field::OrderID, order_id.unwrap())
                },
                ("j", Field::BusinessRejectRefID, cl_ord_id.clone()),
            ])
            .await
        {
            Ok(res) => {
                match res.get_message_type() {
                    "j" => {
                        // failed
                        Err(Error::OrderFailed(
                            res.get_field_value(Field::Text)
                                .unwrap_or("Unknown error".into()),
                        )
                        .into())
                    }
                    _ => {
                        // "8" Success
                        parse_func::parse_execution_report(res)
                    }
                }
            }
            Err(err) => Err(err),
        }
    }

    /// Order cancel reqeuest
    ///
    /// # Arguments
    ///
    /// * `orig_cl_ord_id` - A unique identifier for the order, which is going to be canceled, allocated by the client.
    /// * `order_id` - Unique ID of an order, returned by the server.
    ///
    ///  Either `orig_cl_ord_id` or `order_id` must be passed to this function. If both are `None`, the function will return an error.
    pub async fn cancel_order(
        &self,
        org_cl_ord_id: Option<String>,
        order_id: Option<String>,
    ) -> Result<ExecutionReport, Error> {
        if org_cl_ord_id.is_none() && order_id.is_none() {
            return Err(Error::MissingArgumentError);
        }
        self.check_connection()?;

        let orgid = match org_cl_ord_id.clone() {
            Some(v) => v,
            None => order_id.clone().unwrap(),
        };
        let oid = match order_id {
            Some(v) => v,
            None => org_cl_ord_id.unwrap(),
        };

        let cl_ord_id = self.create_unique_id();
        let req = OrderCancelReq::new(orgid, Some(oid), cl_ord_id.clone());
        self.internal.send_message(req).await?;
        match self
            .fetch_response(vec![
                ("8", Field::ClOrdId, cl_ord_id.clone()),
                ("j", Field::BusinessRejectRefID, cl_ord_id.clone()),
                ("9", Field::ClOrdId, cl_ord_id.clone()),
            ])
            .await
        {
            Ok(res) => {
                match res.get_message_type() {
                    "j" => {
                        // failed
                        Err(Error::OrderFailed(
                            res.get_field_value(Field::Text)
                                .unwrap_or("Unknown error".into()),
                        )
                        .into())
                    }
                    "9" => {
                        // cancel rejected
                        Err(Error::OrderCancelRejected(
                            res.get_field_value(Field::Text)
                                .unwrap_or("Unknown error".into()),
                        )
                        .into())
                    }
                    _ => {
                        // "8" Success
                        parse_func::parse_execution_report(res)
                    }
                }
            }
            Err(err) => Err(err),
        }
    }
}
