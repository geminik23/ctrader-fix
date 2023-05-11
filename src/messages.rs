use crate::types::{Config, Field, OrderType, Side, SubID};
use chrono::Utc;
use std::collections::HashMap;

// Response
#[derive(Debug, Clone)]
pub struct ResponseMessage {
    message: String,
    // fields: HashMap<u32, String>,
    field_idx: HashMap<u32, usize>,
    fields: Vec<(u32, String)>,
}

impl ResponseMessage {
    pub fn new(message: &str, delimiter: &str) -> Self {
        let message = message.replace(delimiter, "|");
        let mut idx_map = HashMap::new();
        let fields = message
            .split("|")
            .filter(|field| !field.is_empty() && field.contains("="))
            .enumerate()
            .map(|(idx, field)| {
                let parts = field.split("=").collect::<Vec<_>>();

                let field = parts[0].parse::<u32>().unwrap();
                if !idx_map.contains_key(&field) {
                    idx_map.insert(parts[0].parse::<u32>().unwrap(), idx);
                }
                (field, parts[1].to_string())
            })
            .collect::<Vec<_>>();
        Self {
            message,
            field_idx: idx_map,
            fields,
        }
    }

    pub fn get_field_value(&self, field: Field) -> Option<String> {
        self.field_idx
            .get(&(field as u32))
            .map(|idx| self.fields[*idx].1.clone())
        // self.fields.get(&(field as u32)).map(|v| v.clone())
    }

    pub fn get_message_type(&self) -> &str {
        self.field_idx
            .get(&(Field::MsgType as u32))
            .map(|idx| &self.fields[*idx].1)
            .unwrap()
    }

    pub fn get_message(&self) -> &str {
        &self.message
    }

    pub fn get_repeating_groups(
        &self,
        count_key: Field,
        start_field: Field,
        end_field: Option<Field>,
    ) -> Vec<HashMap<Field, String>> {
        // FIX later : better algorithms
        let count_key = count_key as u32;

        let mut count = None;
        let mut result = vec![];
        let mut item = HashMap::new();
        // located more than 8
        if let Some(start_idx) = self.field_idx.get(&(count_key as u32)) {
            for (k, v) in &self.fields[*start_idx..] {
                match count {
                    Some(0) => {
                        return result;
                    }
                    None => {
                        if *k == count_key as u32 {
                            count = Some(v.parse::<usize>().unwrap());
                        }
                        continue;
                    }
                    _ => {
                        let key = Field::try_from(*k)
                            .expect("Failed to parse u32 to Field in get_repeating_groups");

                        match end_field {
                            Some(end_key) => {
                                item.insert(key, v.clone());

                                if key == end_key {
                                    result.push(item.clone());
                                    item.clear();
                                    count = count.map(|c| c - 1);
                                }
                            }
                            None => {
                                if key == start_field && !item.is_empty() {
                                    result.push(item.clone());
                                    item.clear();
                                    count = count.map(|c| c - 1);
                                }

                                item.insert(key, v.clone());
                            }
                        }
                        // if Some(key) == end_field
                        //     || (end_field.is_none() && key == start_field && !item.is_empty())
                        // {
                        // }
                        // item.insert(key, v.clone());
                    }
                }
            }
            result.push(item.clone());
        }
        result
    }
}

fn format_field<T: std::fmt::Display>(field: Field, value: T) -> String {
    format!("{}={}", field as u32, value)
}
// Request

// motivated from the cTraderFixPy .
pub trait RequestMessage: Send {
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
pub struct LogoutReq;

impl RequestMessage for LogoutReq {
    fn get_body(&self, _delimiter: &str, _config: &Config) -> Option<String> {
        None
    }

    fn get_message_type(&self) -> &str {
        "5"
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

#[derive(Debug, Clone, Default)]
pub struct TestReq {
    test_req_id: String,
}

impl TestReq {
    pub fn new(test_req_id: String) -> Self {
        Self { test_req_id }
    }
}

impl RequestMessage for TestReq {
    fn get_body(&self, _delimiter: &str, _config: &Config) -> Option<String> {
        Some(format_field(Field::TestReqID, &self.test_req_id))
    }

    fn get_message_type(&self) -> &str {
        "1"
    }
}

#[derive(Debug, Clone, Default)]
pub struct ResendReq {
    begin_seq_no: u32,
    end_seq_no: u32,
}

impl ResendReq {
    pub fn new(begin_seq_no: u32, end_seq_no: u32) -> Self {
        Self {
            begin_seq_no,
            end_seq_no,
        }
    }
}

impl RequestMessage for ResendReq {
    fn get_body(&self, delimiter: &str, _config: &Config) -> Option<String> {
        let fields = vec![
            format_field(Field::BeginSeqNo, self.begin_seq_no),
            format_field(Field::EndSeqNo, self.end_seq_no),
        ];
        Some(fields.join(delimiter))
    }

    fn get_message_type(&self) -> &str {
        "2"
    }
}

#[derive(Debug, Clone, Default)]
pub struct SequenceReset {
    gap_fill_flag: Option<bool>,
    new_seq_no: u32,
}

impl SequenceReset {
    pub fn new(gap_fill_flag: Option<bool>, new_seq_no: u32) -> Self {
        Self {
            gap_fill_flag,
            new_seq_no,
        }
    }
}

impl RequestMessage for SequenceReset {
    fn get_body(&self, delimiter: &str, _config: &Config) -> Option<String> {
        let mut fields = vec![format_field(Field::NewSeqNo, self.new_seq_no)];

        if let Some(gap_fill_flag) = self.gap_fill_flag {
            fields.push(format_field(Field::GapFillFlag, gap_fill_flag));
        }

        Some(fields.join(delimiter))
    }

    fn get_message_type(&self) -> &str {
        "4"
    }
}

#[derive(Debug, Clone, Default)]
pub struct MarketDataReq {
    md_req_id: String,
    subscription_req_type: char,
    market_depth: u32,
    md_update_type: Option<u32>,
    no_md_entry_types: u32,
    md_entry_type: Vec<char>,
    no_related_sym: u32,
    symbol: u32,
}

impl MarketDataReq {
    pub fn new(
        md_req_id: String,
        subscription_req_type: char,
        market_depth: u32,
        md_update_type: Option<u32>,
        md_entry_type: &[char],
        no_related_sym: u32,
        symbol: u32,
    ) -> Self {
        Self {
            md_req_id,
            subscription_req_type,
            market_depth,
            md_update_type,
            no_md_entry_types: md_entry_type.len() as u32,
            md_entry_type: md_entry_type.into(),
            no_related_sym,
            symbol,
        }
    }
}

impl RequestMessage for MarketDataReq {
    fn get_body(&self, delimiter: &str, _config: &Config) -> Option<String> {
        let mut fields = vec![
            format_field(Field::MDReqID, &self.md_req_id),
            format_field(Field::SubscriptionRequestType, self.subscription_req_type),
            format_field(Field::MarketDepth, self.market_depth),
            format_field(Field::NoMDEntryTypes, self.no_md_entry_types),
        ];
        // order important
        self.md_entry_type
            .iter()
            .for_each(|c| fields.push(format_field(Field::MDEntryType, c)));

        fields.extend([
            format_field(Field::NoRelatedSym, self.no_related_sym),
            format_field(Field::Symbol, &self.symbol),
        ]);

        if let Some(md_update_type) = self.md_update_type {
            fields.push(format_field(Field::MDUpdateType, md_update_type));
        }

        Some(fields.join(delimiter))
    }

    fn get_message_type(&self) -> &str {
        "V"
    }
}

#[derive(Debug, Clone, Default)]
pub struct NewOrderSingleReq {
    pub cl_ord_id: String,
    pub symbol: String,
    pub side: Side,
    pub transact_time: Option<chrono::NaiveDateTime>,
    pub order_qty: f64,
    pub ord_type: OrderType,
    pub price: Option<f64>,
    pub stop_px: Option<f64>,
    pub expire_time: Option<chrono::NaiveDateTime>,
    pub pos_maint_rpt_id: Option<String>,
    pub designation: Option<String>,
}

impl NewOrderSingleReq {
    pub fn new(
        cl_ord_id: String,
        symbol: String,
        side: Side,
        transact_time: Option<chrono::NaiveDateTime>,
        order_qty: f64,
        ord_type: OrderType,
        price: Option<f64>,
        stop_px: Option<f64>,
        expire_time: Option<chrono::NaiveDateTime>,
        pos_maint_rpt_id: Option<String>,
        designation: Option<String>,
    ) -> Self {
        Self {
            cl_ord_id,
            symbol,
            side,
            transact_time,
            order_qty,
            ord_type,
            price,
            stop_px,
            expire_time,
            pos_maint_rpt_id,
            designation,
        }
    }
}

impl RequestMessage for NewOrderSingleReq {
    fn get_body(&self, delimiter: &str, _config: &Config) -> Option<String> {
        let mut fields = vec![
            format_field(Field::ClOrdId, &self.cl_ord_id),
            format_field(Field::Symbol, &self.symbol),
            format_field(Field::Side, self.side as u32),
            format_field(
                Field::TransactTime,
                self.transact_time.map_or_else(
                    || Utc::now().format("%Y%m%d-%H:%M:%S").to_string(),
                    |d| d.format("%Y%m%d-%H:%M:%S").to_string(),
                ),
            ),
            format_field(Field::OrderQty, self.order_qty),
            format_field(Field::OrdType, self.ord_type as u32),
        ];

        if let Some(price) = self.price {
            fields.push(format_field(Field::Price, price));
        }
        if let Some(stop_px) = self.stop_px {
            fields.push(format_field(Field::StopPx, stop_px));
        }
        if let Some(expire_time) = self.expire_time {
            fields.push(format_field(
                Field::ExpireTime,
                expire_time.format("%Y%m%d-%H:%M:%S"),
            ));
        }
        if let Some(pos_maint_rpt_id) = &self.pos_maint_rpt_id {
            fields.push(format_field(Field::PosMaintRptID, pos_maint_rpt_id));
        }
        if let Some(designation) = &self.designation {
            fields.push(format_field(Field::Designation, designation));
        }

        Some(fields.join(delimiter))
    }

    fn get_message_type(&self) -> &str {
        "D"
    }
}

#[derive(Debug, Clone, Default)]
pub struct OrderStatusReq {
    pub cl_ord_id: String,
    pub side: Option<Side>,
}

impl RequestMessage for OrderStatusReq {
    fn get_body(&self, delimiter: &str, _config: &Config) -> Option<String> {
        let mut fields = vec![format_field(Field::ClOrdId, &self.cl_ord_id)];

        if let Some(side) = self.side {
            fields.push(format_field(Field::Side, side as u32));
        }

        Some(fields.join(delimiter))
    }

    fn get_message_type(&self) -> &str {
        "H"
    }
}

impl OrderStatusReq {
    pub fn new(cl_ord_id: String, side: Option<Side>) -> Self {
        Self { cl_ord_id, side }
    }
}

#[derive(Debug, Clone, Default)]
pub struct OrderMassStatusReq {
    pub mass_status_req_id: String,
    pub mass_status_req_type: u32,
    pub issue_date: Option<chrono::NaiveDateTime>,
}

impl RequestMessage for OrderMassStatusReq {
    fn get_body(&self, delimiter: &str, _config: &Config) -> Option<String> {
        let mut fields = vec![
            format_field(Field::MassStatusReqID, &self.mass_status_req_id),
            format_field(Field::MassStatusReqType, self.mass_status_req_type),
        ];

        if let Some(issue_date) = self.issue_date {
            fields.push(format_field(
                Field::IssueDate,
                issue_date.format("%Y%m%d-%H:%M:%S"),
            ));
        }

        Some(fields.join(delimiter))
    }

    fn get_message_type(&self) -> &str {
        "AF"
    }
}

#[derive(Debug, Clone, Default)]
pub struct PositionsReq {
    pub pos_req_id: String,
    pub pos_maint_rpt_id: Option<String>,
}

impl PositionsReq {
    pub fn new(pos_req_id: String, pos_maint_rpt_id: Option<String>) -> Self {
        Self {
            pos_req_id,
            pos_maint_rpt_id,
        }
    }
}

impl RequestMessage for PositionsReq {
    fn get_body(&self, delimiter: &str, _config: &Config) -> Option<String> {
        let mut fields = vec![format_field(Field::PosReqID, &self.pos_req_id)];

        if let Some(pos_maint_rpt_id) = &self.pos_maint_rpt_id {
            fields.push(format_field(Field::PosMaintRptID, pos_maint_rpt_id));
        }

        Some(fields.join(delimiter))
    }

    fn get_message_type(&self) -> &str {
        "AN"
    }
}

#[derive(Debug, Clone, Default)]
pub struct OrderCancelReq {
    pub orig_cl_ord_id: String,
    pub order_id: Option<String>,
    pub cl_ord_id: String,
}
impl OrderCancelReq {
    pub fn new(orig_cl_ord_id: String, order_id: Option<String>, cl_ord_id: String) -> Self {
        Self {
            orig_cl_ord_id,
            order_id,
            cl_ord_id,
        }
    }
}
impl RequestMessage for OrderCancelReq {
    fn get_body(&self, delimiter: &str, _config: &Config) -> Option<String> {
        let mut fields = vec![
            format_field(Field::OrigClOrdID, &self.orig_cl_ord_id),
            format_field(Field::ClOrdId, &self.cl_ord_id),
        ];

        if let Some(order_id) = &self.order_id {
            fields.push(format_field(Field::OrderID, order_id));
        }

        Some(fields.join(delimiter))
    }

    fn get_message_type(&self) -> &str {
        "F"
    }
}

#[derive(Debug, Clone, Default)]
pub struct OrderCancelReplaceReq {
    pub orig_cl_ord_id: String,
    pub order_id: Option<String>,
    pub cl_ord_id: String,
    pub order_qty: f64,
    pub price: Option<f64>,
    pub stop_px: Option<f64>,
    pub expire_time: Option<chrono::NaiveDateTime>,
}

impl OrderCancelReplaceReq {
    pub fn new(
        orig_cl_ord_id: String,
        order_id: Option<String>,
        cl_ord_id: String,
        order_qty: f64,
        price: Option<f64>,
        stop_px: Option<f64>,
        expire_time: Option<chrono::NaiveDateTime>,
    ) -> Self {
        Self {
            orig_cl_ord_id,
            order_id,
            cl_ord_id,
            order_qty,
            price,
            stop_px,
            expire_time,
        }
    }
}

impl RequestMessage for OrderCancelReplaceReq {
    fn get_body(&self, delimiter: &str, _config: &Config) -> Option<String> {
        let mut fields = vec![
            format_field(Field::OrigClOrdID, &self.orig_cl_ord_id),
            format_field(Field::ClOrdId, &self.cl_ord_id),
            format_field(Field::OrderQty, self.order_qty),
        ];

        if let Some(order_id) = &self.order_id {
            fields.push(format_field(Field::OrderID, order_id));
        }
        if let Some(price) = self.price {
            fields.push(format_field(Field::Price, price));
        }
        if let Some(stop_px) = self.stop_px {
            fields.push(format_field(Field::StopPx, stop_px));
        }
        if let Some(expire_time) = self.expire_time {
            fields.push(format_field(
                Field::ExpireTime,
                expire_time.format("%Y%m%d-%H:%M:%S"),
            ));
        }

        Some(fields.join(delimiter))
    }

    fn get_message_type(&self) -> &str {
        "G"
    }
}

#[derive(Debug, Clone, Default)]
pub struct SecurityListReq {
    pub security_req_id: String,
    pub security_list_req_type: u32,
    pub symbol: Option<String>,
}

impl SecurityListReq {
    pub fn new(
        security_req_id: String,
        security_list_req_type: u32,
        symbol: Option<String>,
    ) -> Self {
        Self {
            security_req_id,
            security_list_req_type,
            symbol,
        }
    }
}

impl RequestMessage for SecurityListReq {
    fn get_body(&self, delimiter: &str, _config: &Config) -> Option<String> {
        let mut fields = vec![
            format_field(Field::SecurityReqID, &self.security_req_id),
            format_field(Field::SecurityListRequestType, self.security_list_req_type),
        ];

        if let Some(symbol) = &self.symbol {
            fields.push(format_field(Field::Symbol, symbol));
        }

        Some(fields.join(delimiter))
    }

    fn get_message_type(&self) -> &str {
        "x"
    }
}

#[cfg(test)]
mod tests {
    use super::ResponseMessage;
    use crate::types::{Field, DELIMITER};
    #[test]
    fn test_parse_repeating_group() {
        let res = "8=FIX.4.4|9=134|35=W|34=2|49=CSERVER|50=QUOTE|52=20170117-10:26:54.630|56=live.theBroker.12345|57=any_string|55=1|268=2|269=0|270=1.06625|269=1|270=1.0663|10=118|".to_string().replace("|", DELIMITER);
        let msg = ResponseMessage::new(&res, DELIMITER);
        let result = msg.get_repeating_groups(
            Field::NoMDEntries,
            Field::MDEntryType,
            Some(Field::MDEntryPx),
        );

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].len(), 2);
        assert_eq!(result[1].len(), 2);
    }
}
