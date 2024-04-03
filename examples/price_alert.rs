use cfix::utilities::price_alert::{AlertSet, PriceAlert};

fn main() {
    let mut price_alert = PriceAlert::new();

    // Example usage of setting a high price alert for an example "EURUSD" symbol, ID: 1.
    let symbol_id = 1;
    let high_price_alert = AlertSet::High(1.2345);

    let alert_id = price_alert.set_alert(symbol_id, high_price_alert, None);
    println!("High price alert set with ID: {}", alert_id);

    let new_price = (1.2350, 1.2355); // Assuming (bid, ask)
    if let Some(alerts_triggered) = price_alert.on_price(symbol_id, new_price) {
        for alert_id in alerts_triggered {
            println!("Alert triggered: {}", alert_id);
        }
    }
}
