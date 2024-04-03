use std::collections::HashMap;

use nanoid::nanoid;

#[derive(Debug, Clone, PartialEq)]
pub enum AlertSet {
    High(f64),
    Low(f64),
}

/// A `PriceAlert` struct for setting up price alerts on trading symbols.
///
/// This struct allows to set high and low price alerts for specific trading symbols.
/// Alerts are triggered based on the bid price of the symbol.
pub struct PriceAlert {
    // For each symbol_id with its alert type (high or low)
    alert_price: HashMap<u32, HashMap<String, AlertSet>>,
    // Maps each alert ID to its corresponding symbol_id
    id2symbol: HashMap<String, u32>,
    // Stores the current price for symbol_id (bid, ask)
    price: HashMap<u32, (f64, f64)>,
}

impl PriceAlert {
    pub fn new() -> Self {
        Self {
            alert_price: HashMap::new(),
            price: HashMap::new(),
            id2symbol: HashMap::new(),
        }
    }
    pub fn get_price(&self, symbol_id: u32) -> Option<f64> {
        self.price.get(&symbol_id).map(|(b, _)| *b)
    }

    pub fn on_price(&mut self, symbol_id: u32, price: (f64, f64)) -> Option<Vec<String>> {
        let Some(alert_list) = self.alert_price.get_mut(&symbol_id) else {
            return None;
        };
        let old_price = self.price.insert(symbol_id, price.clone()).unwrap_or(price);
        let ids = alert_list
            .iter()
            .map(|(id, alertset)| match alertset {
                AlertSet::High(p) => (id, *p <= old_price.0 || *p <= price.0),
                AlertSet::Low(p) => (id, *p >= old_price.0 || *p >= price.0),
            })
            .filter(|(_, b)| *b)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();

        ids.iter().for_each(|id| {
            let _alertset = alert_list.remove(id);
            self.id2symbol.remove(id);
        });

        if ids.is_empty() {
            return None;
        }
        Some(ids)
    }

    /// Modify the price
    ///
    /// * Returns the new AlertSet
    pub fn modify_price(&mut self, alert_id: String, price: f64) -> Option<AlertSet> {
        if let Some(symbol) = self.id2symbol.get(&alert_id) {
            if let Some(cont) = self.alert_price.get_mut(&symbol) {
                if let Some(alert) = cont.remove(&alert_id) {
                    let new_alert = match alert {
                        AlertSet::Low(_) => AlertSet::Low(price),
                        AlertSet::High(_) => AlertSet::High(price),
                    };
                    cont.insert(alert_id, new_alert.clone());
                    return Some(new_alert);
                }
            }
        }
        None
    }

    pub fn remove(&mut self, alert_id: String) -> Option<AlertSet> {
        if let Some(symbol) = self.id2symbol.remove(&alert_id) {
            if let Some(cont) = self.alert_price.get_mut(&symbol) {
                return cont.remove(&alert_id);
            }
        }
        None
    }

    pub fn set_alert(&mut self, symbol_id: u32, set: AlertSet, alert_id: Option<String>) -> String {
        // FIXME later check the alert_id
        //
        let alert_id = alert_id.unwrap_or(nanoid!(10));

        self.id2symbol.insert(alert_id.clone(), symbol_id);
        self.alert_price
            .entry(symbol_id)
            .or_insert(HashMap::new())
            .insert(alert_id.clone(), set);

        alert_id
    }
}
