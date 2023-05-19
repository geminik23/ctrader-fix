use chrono::NaiveDateTime;
use serde::Deserialize;
use std::{collections::HashMap, str::FromStr, sync::Arc};

use async_trait::async_trait;
use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::messages::ResponseMessage;

pub const DELIMITER: &str = "\u{1}";

#[async_trait]
pub trait ConnectionHandler {
    async fn on_connect(&self);
    async fn on_logon(&self);
    async fn on_disconnect(&self);
}

#[async_trait]
pub trait MarketDataHandler {
    /// Called when a new price for a symbol is received.
    async fn on_price_of(&self, symbol_id: u32, price: SpotPrice);

    /// Called when a full refresh of the market depth is received.
    async fn on_market_depth_full_refresh(
        &self,
        symbol_id: u32,
        full_depth: HashMap<String, DepthPrice>,
    );

    /// Called when an incremental update to the market depth is received.
    async fn on_market_depth_incremental_refresh(&self, refresh: Vec<IncrementalRefresh>);

    /// Called when a spot subscription request has been accepted by the server.
    /// This function has a default empty implementation and can be overridden by the struct implementing this trait.
    async fn on_accpeted_spot_subscription(&self, symbol_id: u32) {}

    /// Called when a depth subscription request has been accepted by the server.
    /// This function has a default empty implementation and can be overridden by the struct implementing this trait.
    async fn on_accpeted_depth_subscription(&self, symbol_id: u32) {}

    /// Called when a spot subscription request has been rejected by the server.
    /// This function has a default empty implementation and can be overridden by the struct implementing this trait.
    async fn on_rejected_spot_subscription(&self, symbol_id: u32, err_msg: String) {}

    /// Called when a depth subscription request has been rejected by the server.
    /// This function has a default empty implementation and can be overridden by the struct implementing this trait.
    async fn on_rejected_depth_subscription(&self, symbol_id: u32, err_msg: String) {}
}

#[async_trait]
pub trait TradeDataHandler {
    async fn on_execution_report(&self, exec_report: ExecutionReport);
}

// == Trade type definitions
#[derive(Debug)]
pub struct SymbolInformation {
    pub id: u32,
    pub name: String,
    pub digits: u32,
}

#[derive(Debug)]
pub struct PositionReport {
    pub symbol_id: u32,
    pub position_id: String,
    pub long_qty: f64,
    pub short_qty: f64,
    pub settle_price: f64,
    pub absolute_tp: Option<f64>,
    pub absolute_sl: Option<f64>,
    pub trailing_sl: Option<bool>,
    pub trigger_method_sl: Option<u32>,
    pub guaranteed_sl: Option<bool>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ExecutionType {
    OrderStatus,
    New,
    Canceled,
    Replace,
    Rejected,
    Expired,
    Trade,
}

impl FromStr for ExecutionType {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0" => Ok(ExecutionType::New),
            "4" => Ok(ExecutionType::Canceled),
            "5" => Ok(ExecutionType::Replace),
            "8" => Ok(ExecutionType::Rejected),
            "C" => Ok(ExecutionType::Expired),
            "F" => Ok(ExecutionType::Trade),
            "I" => Ok(ExecutionType::OrderStatus),
            _ => Err(ParseError(s.into())),
        }
    }
}

#[derive(Debug)]
pub struct ExecutionReport {
    /// 150.
    pub exec_type: ExecutionType,
    pub order_report: OrderReport,
}

#[derive(Debug)]
pub struct OrderReport {
    /// Instrument identificators are provided by Spotware. 55
    pub symbol: u32,

    /// cTrader order id. 37
    pub order_id: String,

    /// Unique identifier for the order, allocated by the client. 11
    pub cl_ord_id: String,

    /// Position ID. 72
    pub pos_main_rept_id: String,

    /// Client custom order label. 494
    pub designation: Option<String>,

    /// Order status. 39
    pub order_status: OrderStatus,

    /// Order type : Marekt, Limit, Stop, Stop limit. 40
    pub order_type: OrderType,

    /// 54.
    pub side: Side,

    //** price **//
    /// If supplied in the NewOrderSingle, it is echoed back in this ExecutionReport. 44
    pub price: Option<f64>,

    /// If supplied in the NewOrderSingle, it is echoed back in this ExecutionReport. 99
    pub stop_px: Option<f64>,

    /// The price at which the deal was filled. For an IOC or GTD order, this is the VWAP (Volume Weighted Average Price) of the filled order. 6
    pub avx_px: Option<f64>,

    /// The absolute price at which Take Profit will be triggered. 1000
    pub absolute_tp: Option<f64>,

    /// The distance in pips from the entry price at which the Take Profit will be triggered. 1001
    pub reltative_tp: Option<f64>,

    /// The absolute price at which Stop Loss will be triggered. 1002
    pub absolute_sl: Option<f64>,

    /// The distance in pips from the entry price at which the Stop Loss will be triggered. 1003
    pub reltative_sl: Option<f64>,

    /// Indicates if Stop Loss is trailing. 1004
    pub trailing_sl: Option<bool>,

    /// Indicated trigger method of the Stop Loss. 1005
    ///
    /// 1 = The Stop Loss will be triggered by the trade side.
    /// 2 = The stop loss will be triggered by the opposite side (Ask for Buy positions and by Bid for Sell positions),
    /// 3 = Stop Loss will be triggered after two consecutive ticks according to the trade side.
    /// 4 = Stop Loss will be triggered after two consecutive ticks according to the opposite side (second Ask tick for Buy positions and second Bid tick for Sell positions).
    pub trigger_method_sl: Option<u32>,
    /// Indicates if Stop Loss is guaranteed. 1006
    pub guaranteed_sl: Option<bool>,

    /// The total amount of the order which has been filled. 14
    pub cum_qty: Option<f64>,
    /// Number of shares ordered. This represents the number of shares for equities or based on normal convention the number of contracts for options, futures, convertible bonds, etc. 38
    pub order_qty: f64,
    /// The amount of the order still to be filled. This is a value between 0 (fully filled) and OrderQty (partially filled). 151
    pub leaves_qty: f64,
    /// The bought/sold amount of the order which has been filled on this (last) fill. 32
    pub last_qty: Option<f64>,

    // FIXME new type?
    /// 59
    /// 1 = Good Till Cancel (GTC);
    /// 3 = Immediate Or Cancel (IOC);
    /// 6 = Good Till Date (GTD).
    pub time_in_force: String,

    /// Time the transaction represented by this ExecutionReport occurred message (in UTC). 60
    pub transact_time: NaiveDateTime,

    /// If supplied in the NewOrderSingle, it is echoed back in this ExecutionReport. 126
    pub expire_time: Option<NaiveDateTime>,

    /// Where possible, message to explain execution report. 58
    pub text: Option<String>,
    // /// 103
    // pub ord__rej_reason: Option<u32>,
    // /// MassStatusReqID. 584
    // pub masss_status_req_id: Option<String>,
}

#[repr(u32)]
#[derive(Debug, TryFromPrimitive)]
pub enum TimeInForce {
    GoodTillCancel = 1,
    ImmediateOrCancel = 3,
    GoodTillDate = 6,
}

#[derive(Debug)]
pub enum OrderStatus {
    New,
    ParitallyFilled,
    Filled,
    Rejected,
    Cancelled,
    Expired,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseError(String);

impl FromStr for OrderStatus {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<OrderStatus, Self::Err> {
        match s {
            "0" => Ok(Self::New),
            "1" => Ok(Self::ParitallyFilled),
            "2" => Ok(Self::Filled),
            "8" => Ok(Self::Rejected),
            "4" => Ok(Self::Cancelled),
            "C" => Ok(Self::Expired),
            _ => Err(ParseError(s.into())),
        }
    }
}

// == Market type definition

#[derive(Debug)]
pub enum MarketType {
    Spot,
    Depth,
}

impl std::fmt::Display for MarketType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Spot => "Spot",
            Self::Depth => "Depth",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone)]
pub enum PriceType {
    Bid,
    Ask,
}

impl FromStr for PriceType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0" => Ok(Self::Bid),
            "1" => Ok(Self::Ask),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpotPrice {
    pub bid: f64,
    pub ask: f64,
}

#[derive(Debug, Clone)]
pub struct DepthPrice {
    pub price_type: PriceType,
    pub price: f64,
    pub size: f64,
}

#[derive(Debug, Clone)]
pub enum IncrementalRefresh {
    New {
        symbol_id: u32,
        entry_id: String,
        data: DepthPrice,
    },
    Delete {
        symbol_id: u32,
        entry_id: String,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // connection errors
    #[error("No connection")]
    NotConnected,
    #[error("logged out")]
    LoggedOut,

    #[error("Field not found : {0}")]
    FieldNotFoundError(Field),

    #[error("Missing argument error")]
    MissingArgumentError,

    // #[error("Request failed")]
    // RequestFailed,
    #[error("Order request failed : {0}")]
    OrderFailed(String), // for "j"
    #[error("Order cancel rejected : {0}")]
    OrderCancelRejected(String),

    // subscription errors for market client
    #[error("Failed to {2} subscription {0}: {1}")]
    SubscriptionError(u32, String, MarketType),
    #[error("Already subscribed {1} for symbol({0})")]
    SubscribedAlready(u32, MarketType),
    #[error("Waiting then response of {1} subscription for symbol({0})")]
    RequestingSubscription(u32, MarketType),
    #[error("Not susbscribed {1} for symbol({0})")]
    NotSubscribed(u32, MarketType),

    #[error("Timeout error")]
    TimeoutError,

    // internal errors
    #[error("Request rejected - {0}")]
    RequestRejected(String),
    #[error("Failed to find the response")]
    NoResponse,
    #[error("Unknown errro")]
    UnknownError,

    // reponse send error
    #[error(transparent)]
    SendError(#[from] async_std::channel::SendError<ResponseMessage>),

    // #[error(transparent)]
    // TriggerError(#[from] async_std::channel::SendError<String>),
    #[error(transparent)]
    RecvError(#[from] async_std::channel::RecvError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

//
// only for internal
pub type MarketCallback = Arc<dyn Fn(InternalMDResult) -> () + Send + Sync>;
pub type TradeCallback = Arc<dyn Fn(ResponseMessage) -> () + Send + Sync>;
//

pub enum InternalMDResult {
    MD {
        msg_type: char,
        symbol_id: u32,
        data: Vec<HashMap<Field, String>>,
    },
    MDReject {
        symbol_id: u32,
        md_req_id: String,
        err_msg: String,
    },
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub host: String,
    pub username: String,
    pub password: String,
    pub sender_comp_id: String,
    pub heart_beat: u32,
}

impl Config {
    pub fn new(
        host: String,
        username: String,
        password: String,
        sender_comp_id: String,
        heart_beat: u32,
    ) -> Self {
        Self {
            host,
            username,
            password,
            sender_comp_id,
            heart_beat,
        }
    }
}
#[repr(u32)]
#[derive(Debug, PartialEq, TryFromPrimitive, IntoPrimitive, Clone, Eq, Hash, Copy)]
pub enum Field {
    AvgPx = 6,
    BeginSeqNo = 7,
    BeginString = 8,
    BodyLength = 9,
    CheckSum = 10,
    ClOrdId = 11,
    CumQty = 14,
    EndSeqNo = 16,
    OrdQty = 32,
    MsgSeqNum = 34,
    MsgType = 35,
    NewSeqNo = 36,
    OrderID = 37,
    OrderQty = 38,
    OrdStatus = 39,
    OrdType = 40,
    OrigClOrdID = 41,
    Price = 44,
    RefSeqNum = 45,
    SenderCompID = 49,
    SenderSubID = 50,
    SendingTime = 52,
    Side = 54,
    Symbol = 55,
    TargetCompID = 56,
    TargetSubID = 57,
    Text = 58,
    TimeInForce = 59,
    TransactTime = 60,
    EncryptMethod = 98,
    StopPx = 99,
    OrdRejReason = 103,
    HeartBtInt = 108,
    TestReqID = 112,
    GapFillFlag = 123,
    ExpireTime = 126,
    ResetSeqNumFlag = 141,
    NoRelatedSym = 146,
    ExecType = 150,
    LeavesQty = 151,
    IssueDate = 225,
    MDReqID = 262,
    SubscriptionRequestType = 263,
    MarketDepth = 264,
    MDUpdateType = 265,
    NoMDEntryTypes = 267,
    NoMDEntries = 268,
    MDEntryType = 269,
    MDEntryPx = 270,
    MDEntrySize = 271,
    MDEntryID = 278,
    MDUpdateAction = 279,
    SecurityReqID = 320,
    SecurityResponseID = 322,
    EncodedTextLen = 354,
    EncodedText = 355,
    RefTagID = 371,
    RefMsgType = 372,
    SessionRejectReason = 373,
    BusinessRejectRefID = 379,
    BusinessRejectReason = 380,
    CxlRejResponseTo = 434,
    Designation = 494,
    Username = 553,
    Password = 554,
    SecurityListRequestType = 559,
    SecurityRequestResult = 560,
    MassStatusReqID = 584,
    MassStatusReqType = 585,
    NoPositions = 702,
    LongQty = 704,
    ShortQty = 705,
    PosReqID = 710,
    PosMaintRptID = 721,
    TotalNumPosReports = 727,
    PosReqResult = 728,
    SettlPrice = 730,
    TotNumReports = 911,
    AbsoluteTP = 1000,
    RelativeTP = 1001,
    AbsoluteSL = 1002,
    RelativeSL = 1003,
    TrailingSL = 1004,
    TriggerMethodSL = 1005,
    GuaranteedSL = 1006,
    SymbolName = 1007,
    SymbolDigits = 1008,
}
impl std::fmt::Display for Field {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{:?}", self))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum SubID {
    QUOTE,
    TRADE,
}

impl std::fmt::Display for SubID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SubID::QUOTE => "QUOTE",
            SubID::TRADE => "TRADE",
        };
        f.write_str(s)
    }
}

impl FromStr for SubID {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "QUOTE" => Ok(SubID::QUOTE),
            "TRADE" => Ok(SubID::TRADE),
            _ => Err(()),
        }
    }
}

#[repr(u32)]
#[derive(Debug, PartialEq, TryFromPrimitive, Clone, Copy)]
pub enum Side {
    BUY = 1,
    SELL = 2,
}

impl Default for Side {
    fn default() -> Self {
        Side::BUY
    }
}

#[repr(u32)]
#[derive(Debug, PartialEq, TryFromPrimitive, Clone, Copy)]
pub enum OrderType {
    MARKET = 1,
    LIMIT = 2,
    STOP = 3,
    STOP_LIMIT = 4,
}

impl FromStr for OrderType {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" => Ok(Self::MARKET),
            "2" => Ok(Self::LIMIT),
            "3" => Ok(Self::STOP),
            "4" => Ok(Self::STOP_LIMIT),
            _ => Err(ParseError(s.into())),
        }
    }
}

impl Default for OrderType {
    fn default() -> Self {
        OrderType::MARKET
    }
}
