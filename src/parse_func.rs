use chrono::NaiveDateTime;

use crate::{
    messages::ResponseMessage,
    types::{
        Error, Field, NewOrderReport, OrderStatus, OrderStatusReport, OrderType, PositionReport,
        Side, SymbolInformation, DELIMITER,
    },
};

pub fn parse_security_list(res: &ResponseMessage) -> Result<Vec<SymbolInformation>, Error> {
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

pub fn parse_positions(res: &ResponseMessage) -> Result<Vec<PositionReport>, Error> {
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

//
// ORDERs
//

pub fn parse_new_order_report(res: ResponseMessage) -> Result<NewOrderReport, Error> {
    Ok(NewOrderReport {
        symbol: res
            .get_field_value(Field::Symbol)
            .unwrap_or("0".into())
            .parse::<u32>()
            .unwrap(),
        order_qty: res
            .get_field_value(Field::OrderQty)
            .unwrap_or("0.0".into())
            .parse::<f64>()
            .unwrap(),
        order_status: res
            .get_field_value(Field::OrdStatus)
            .map(|v| v.parse::<OrderStatus>().unwrap())
            .unwrap(),
        order_type: res
            .get_field_value(Field::OrdType)
            .map(|v| v.parse::<OrderType>().unwrap())
            .unwrap(),
        side: res
            .get_field_value(Field::Side)
            .map(|v| Side::try_from(v.parse::<u32>().unwrap()).unwrap())
            .unwrap(),
        time_in_force: res.get_field_value(Field::TimeInForce).unwrap(),
        transact_time: res
            .get_field_value(Field::TransactTime)
            .map(|v| NaiveDateTime::parse_from_str(v.as_str(), "%Y%m%d-%H:%M:%S%.3f").unwrap())
            .unwrap(),
        leaves_qty: res
            .get_field_value(Field::LeavesQty)
            .unwrap_or("0.0".into())
            .parse::<f64>()
            .unwrap(),
        pos_main_rept_id: res.get_field_value(Field::PosMaintRptID).unwrap(),
    })
}

pub fn parse_order_status(res: ResponseMessage) -> Result<Vec<OrderStatusReport>, Error> {
    let npos = res
        .get_field_value(Field::TotNumReports)
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
        .map(|res| {
            // match res.into
            //
            OrderStatusReport {
                symbol: res
                    .get_field_value(Field::Symbol)
                    .unwrap_or("0".into())
                    .parse::<u32>()
                    .unwrap(),
                order_id: res.get_field_value(Field::OrderID).unwrap(),
                cum_qty: res
                    .get_field_value(Field::CumQty)
                    .unwrap_or("0.0".into())
                    .parse::<f64>()
                    .unwrap(),
                order_qty: res
                    .get_field_value(Field::OrderQty)
                    .unwrap_or("0.0".into())
                    .parse::<f64>()
                    .unwrap(),
                leaves_qty: res
                    .get_field_value(Field::LeavesQty)
                    .unwrap_or("0.0".into())
                    .parse::<f64>()
                    .unwrap(),
                order_status: res
                    .get_field_value(Field::OrdStatus)
                    .map(|v| v.parse::<OrderStatus>().unwrap())
                    .unwrap(),
                order_type: res
                    .get_field_value(Field::OrdType)
                    .map(|v| v.parse::<OrderType>().unwrap())
                    .unwrap(),
                price: res
                    .get_field_value(Field::Price)
                    .unwrap_or("0.0".into())
                    .parse::<f64>()
                    .unwrap(),
                side: res
                    .get_field_value(Field::Side)
                    .map(|v| Side::try_from(v.parse::<u32>().unwrap()).unwrap())
                    .unwrap(),
                time_in_force: res.get_field_value(Field::TimeInForce).unwrap(),
                transact_time: res
                    .get_field_value(Field::TransactTime)
                    .map(|v| {
                        NaiveDateTime::parse_from_str(v.as_str(), "%Y%m%d-%H:%M:%S%.3f").unwrap()
                    })
                    .unwrap(),
                pos_main_rept_id: res.get_field_value(Field::PosMaintRptID).unwrap(),
            }
        })
        .collect::<Vec<_>>())
}
