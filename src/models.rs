use chrono::Utc;
use serde::Deserialize;
use std::{collections::HashMap, str::FromStr};

use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub host: String,
    username: String,
    password: String,
    broker: String,
    heart_beat: u32,
}

impl Config {
    pub fn new(
        host: String,
        username: String,
        password: String,
        broker: String,
        heart_beat: u32,
    ) -> Self {
        Self {
            host,
            username,
            password,
            broker,
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
    OrdQty = 32,
    MsgSeqNum = 34,
    MsgType = 35,
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
    ExpireTime = 126,
    ResetSeqNumFlag = 141,
    NoRelatedSym = 146,
    ExecType = 150,
    LeavesQty = 151,
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
#[derive(Debug, PartialEq, TryFromPrimitive)]
pub enum Side {
    BUY = 1,
    SELL = 2,
}

#[repr(u32)]
#[derive(Debug, PartialEq, TryFromPrimitive)]
pub enum OrderType {
    MARKET = 1,
    LIMIT = 2,
    STOP = 3,
}

// Response
#[derive(Debug, Clone)]
pub struct ResponseMessage {
    message: String,
    fields: HashMap<Field, String>,
}

impl ResponseMessage {
    pub fn new(message: &str, delimiter: &str) -> Self {
        let message = message.replace(delimiter, "|");
        let fields = message
            .split(delimiter)
            .filter(|field| !field.is_empty() && field.contains("="))
            .map(|field| {
                let parts = field.split("=").collect::<Vec<_>>();
                (
                    Field::try_from(parts[0].parse::<u32>().unwrap()).unwrap(),
                    parts[1].to_string(),
                )
            })
            .collect::<HashMap<_, _>>();
        Self { message, fields }
    }

    pub fn get_field_value(&self, field: Field) -> Option<String> {
        self.fields.get(&field).map(|v| v.clone())
    }

    pub fn get_message_type(&self) -> &str {
        &self.fields.get(&Field::MsgType).unwrap()
    }

    pub fn get_message(&self) -> &str {
        &self.message
    }
}

fn format_field<T: std::fmt::Display>(field: Field, value: T) -> String {
    format!("{}={}", Into::<u32>::into(field), value)
}
// Request

// motivated from the cTraderFixPy .
pub trait RequestMessage {
    fn build(
        &self,
        sub_id: SubID,
        sequence_number: u32,
        delimiter: &str,
        config: &Config,
    ) -> String {
        let body = self.get_body(delimiter, config);
        let header = self.get_header(
            sub_id,
            body.as_ref().map(|s| s.len()).unwrap_or(0),
            sequence_number,
            delimiter,
            config,
        );
        let header_and_body = match body {
            Some(body) => format!("{}{}{}{}", header, delimiter, body, delimiter),
            None => format!("{}{}", header, delimiter),
        };
        let trailer = self.get_trailer(&header_and_body);
        format!("{}{}{}", header_and_body, trailer, delimiter)
    }

    fn get_header(
        &self,
        sub_id: SubID,
        len_body: usize,
        sequence_number: u32,
        delimiter: &str,
        config: &Config,
    ) -> String {
        let fields = vec![
            format_field(Field::MsgType, self.get_message_type()),
            format_field(
                Field::SenderCompID,
                format!("{}.{}", config.broker, config.username),
            ),
            format_field(Field::TargetCompID, "cServer"),
            format_field(Field::TargetSubID, sub_id.to_string()),
            format_field(Field::SenderSubID, sub_id.to_string()),
            format_field(Field::MsgSeqNum, sequence_number),
            format_field(Field::SendingTime, Utc::now().format("%Y%m%d-%H:%M:%S")),
        ];
        let fields_joined = fields.join(delimiter);
        format!(
            "8=FIX.4.4{}9={}{}{}",
            delimiter,
            len_body + fields_joined.len() + 2,
            delimiter,
            fields_joined
        )
    }

    fn get_trailer(&self, header_and_body: &str) -> String {
        let message_bytes = header_and_body.as_bytes();
        let checksum = message_bytes.iter().map(|byte| *byte as u32).sum::<u32>() % 256;
        format!("10={:03}", checksum)
    }

    fn get_body(&self, delimiter: &str, config: &Config) -> Option<String>;

    fn get_message_type(&self) -> &str;
}

#[derive(Debug, Clone, Default)]
pub struct LogonReq {
    pub encryption_scheme: i32,
    pub reset_seq_num: Option<bool>,
}

impl RequestMessage for LogonReq {
    fn get_body(&self, delimiter: &str, config: &Config) -> Option<String> {
        let mut fields = vec![
            format_field(Field::EncryptMethod, self.encryption_scheme),
            format_field(Field::HeartBtInt, config.heart_beat),
            format_field(Field::Username, &config.username),
            format_field(Field::Password, &config.password),
        ];

        match self.reset_seq_num {
            Some(true) => {
                fields.push("141=Y".to_string());
            } // Field::ResetSeqNumFlag
            _ => {}
        }

        Some(fields.join(delimiter))
    }

    fn get_message_type(&self) -> &str {
        "A"
    }
}

#[derive(Debug, Clone, Default)]
pub struct HeartbeatReq {
    test_req_id: Option<String>,
}

impl HeartbeatReq {
    pub fn new(test_req_id: Option<String>) -> Self {
        Self { test_req_id }
    }
}

impl RequestMessage for HeartbeatReq {
    fn get_body(&self, _delimiter: &str, _config: &Config) -> Option<String> {
        self.test_req_id
            .as_ref()
            .map(|test_req_id| format_field(Field::TestReqID, test_req_id))
    }

    fn get_message_type(&self) -> &str {
        "0"
    }
}
