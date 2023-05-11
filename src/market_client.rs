use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use async_std::sync::Mutex;

use crate::{
    fixapi::FixApi,
    messages::MarketDataReq,
    types::{ConnectionHandler, Error, Field, MarketType, PriceType, SpotPrice},
    DepthPrice,
};

#[derive(Debug, PartialEq, Clone, Copy, Eq)]
enum RequestState {
    Requested,
    Accepted,
}

pub struct MarketClient {
    internal: FixApi,

    spot_req_states: Arc<Mutex<HashMap<u32, RequestState>>>,
    depth_req_states: Arc<Mutex<HashMap<u32, RequestState>>>,

    // requested_symbol: Arc<Mutex<HashSet<u32>>>,
    spot_market_data: Arc<Mutex<HashMap<u32, SpotPrice>>>,
    depth_market_data: Arc<Mutex<HashMap<u32, HashMap<String, DepthPrice>>>>, // temporary
}

fn depth_data_from_entries(data: Vec<HashMap<Field, String>>) -> HashMap<String, DepthPrice> {
    let mut depth_data = HashMap::new();
    for e in data.into_iter() {
        let eid = e.get(&Field::MDEntryID).unwrap();
        depth_data.insert(
            eid.clone(),
            DepthPrice {
                price_type: e.get(&Field::MDEntryType).unwrap().parse().unwrap(),
                price: e.get(&Field::MDEntryPx).unwrap().parse::<f64>().unwrap(),
                size: e.get(&Field::MDEntrySize).unwrap().parse::<f64>().unwrap(),
            },
        );
    }
    depth_data
}

fn spot_price_from_market_data(data: Vec<HashMap<Field, String>>) -> SpotPrice {
    let mut price = SpotPrice {
        bid: 0f64,
        ask: 0f64,
    };

    for i in 0..2 {
        let value = data[i]
            .get(&Field::MDEntryPx)
            .unwrap()
            .parse::<f64>()
            .unwrap();

        if data[i].get(&Field::MDEntryType).unwrap() == "0" {
            price.bid = value;
        } else {
            price.ask = value;
        }
    }
    price
}

impl MarketClient {
    pub fn new(
        host: String,
        login: String,
        password: String,
        broker: String,
        heartbeat_interval: Option<u32>,
    ) -> Self {
        let mut internal = FixApi::new(
            crate::types::SubID::QUOTE,
            host,
            login,
            password,
            broker,
            heartbeat_interval,
        );
        let spot_req_states = Arc::new(Mutex::new(HashMap::new()));
        let spot_market_data = Arc::new(Mutex::new(HashMap::new()));

        let depth_req_states = Arc::new(Mutex::new(HashMap::new()));
        let depth_market_data = Arc::new(Mutex::new(HashMap::new()));

        // clone
        let trigger = internal.trigger.clone();
        let spot_req_states_clone = spot_req_states.clone();
        let spot_market_data_clone = spot_market_data.clone();

        let depth_req_states_clone = depth_req_states.clone();
        let depth_market_data_clone = depth_market_data.clone();

        let market_callback =
            move |msg_type: char, symbol_id: u32, data: Vec<HashMap<Field, String>>| {
                // symbol_id is only valid for msg_type - 'W'

                let tx = trigger.clone();
                let spot_req_states_clone = spot_req_states_clone.clone();
                let spot_market_data_clone = spot_market_data_clone.clone();
                let depth_req_states_clone = depth_req_states_clone.clone();
                let depth_market_data_clone = depth_market_data_clone.clone();

                async move {
                    match msg_type {
                        'W' => {
                            // check whether data is spot or depth
                            if data.len() != 0 && !data[0].contains_key(&Field::MDEntryID) {
                                //spot
                                if Some(&RequestState::Requested)
                                    == spot_req_states_clone.lock().await.get(&symbol_id)
                                {
                                    spot_req_states_clone
                                        .lock()
                                        .await
                                        .insert(symbol_id, RequestState::Accepted);

                                    tx.send(()).await.unwrap_or_else(|e| {
                                        // fatal
                                        log::error!(
                                            "Failed to notify that the response is received - {:?}",
                                            e
                                        );
                                    });
                                }

                                // update spot data
                                if data.len() >= 2 {
                                    let prices = spot_price_from_market_data(data);

                                    spot_market_data_clone
                                        .lock()
                                        .await
                                        .insert(symbol_id, prices);

                                    // TODO
                                    // to handler
                                }
                            } else {
                                // depth
                                if Some(&RequestState::Requested)
                                    == depth_req_states_clone.lock().await.get(&symbol_id)
                                {
                                    depth_req_states_clone
                                        .lock()
                                        .await
                                        .insert(symbol_id, RequestState::Accepted);

                                    tx.send(()).await.unwrap_or_else(|e| {
                                        // fatal
                                        log::error!(
                                            "Failed to notify that the response is received - {:?}",
                                            e
                                        );
                                    });
                                }

                                {
                                    // update the depth data
                                    let mut depth_cont = depth_market_data_clone.lock().await;
                                    depth_cont.insert(symbol_id, depth_data_from_entries(data));
                                }

                                // TODO
                                // to handler
                            }
                        }
                        'X' => {
                            // Market data incremental refresh

                            if Some(&RequestState::Accepted)
                                == depth_req_states_clone.lock().await.get(&symbol_id)
                            {
                                //
                            }
                        }
                        _ => {}
                    }
                }
            };
        internal.register_market_callback(market_callback);

        Self {
            internal,
            spot_req_states,
            spot_market_data,
            depth_req_states,
            depth_market_data,
        }
    }

    pub fn register_connection_handler<T: ConnectionHandler + Send + Sync + 'static>(
        &mut self,
        handler: T,
    ) {
        self.internal.register_connection_handler(handler);
    }

    /// Connects to a server
    ///
    /// This method first attempt to establish a connection. If the connection is succesful, then
    /// it proceeds to logon directly.
    pub async fn connect(&mut self) -> Result<(), Error> {
        self.internal.connect().await?;
        self.internal.logon().await
    }

    pub async fn disconnect(&mut self) -> Result<(), Error> {
        self.internal.logout().await?;
        self.internal.disconnect().await
    }

    pub fn is_connected(&self) -> bool {
        self.internal.is_connected()
    }

    pub async fn spot_subscription_list(&self) -> HashSet<u32> {
        self.spot_req_states
            .lock()
            .await
            .iter()
            .filter(|(_, v)| *v == &RequestState::Accepted)
            .map(|(k, _)| *k)
            .collect()
    }

    pub async fn depth_subscription_list(&self) -> HashSet<u32> {
        self.depth_req_states
            .lock()
            .await
            .iter()
            .filter(|(_, v)| *v == &RequestState::Accepted)
            .map(|(k, _)| *k)
            .collect()
    }

    pub async fn price_of(&self, symbol_id: u32) -> Result<SpotPrice, Error> {
        self.spot_market_data
            .lock()
            .await
            .get(&symbol_id)
            .map(|v| v.clone())
            .ok_or(Error::NotSubscribed(symbol_id, MarketType::Spot))
    }

    pub async fn depth_data(&self, symbol_id: u32) -> Result<HashMap<String, DepthPrice>, Error> {
        self.depth_market_data
            .lock()
            .await
            .get(&symbol_id)
            .map(|v| v.clone())
            .ok_or(Error::NotSubscribed(symbol_id, MarketType::Spot))
    }

    pub async fn subscribe_spot(&mut self, symbol_id: u32) -> Result<(), Error> {
        // FIXME later
        // .. code is too messy. is there a better way?

        // check already subscribed?
        // FIXME : two states - requested accepted
        if self.spot_req_states.lock().await.contains_key(&symbol_id) {
            return Err(Error::SubscribedAlready(symbol_id, MarketType::Spot));
        }

        // add to requested symbol.
        self.spot_req_states
            .lock()
            .await
            .insert(symbol_id, RequestState::Requested);

        // intialize the request and send req
        let req = MarketDataReq::new("-1".into(), '1', 1, None, &['0', '1'], 1, symbol_id);
        let seq_num = self.internal.send_message(req).await?;

        // waiting (reject or  marketdata)
        while let Ok(()) = self.internal.wait_notifier().await {
            // check the market data
            if let Some(RequestState::Accepted) = self.spot_req_states.lock().await.get(&symbol_id)
            {
                // accepted
                log::trace!("Spot data subscription accepted for symbol({})", symbol_id);
                break;
            }

            match self.internal.check_req_accepted(seq_num).await {
                // **No accept response for marketdata**
                // Ok(res) => {
                //     // accepted
                //     break;
                // }
                Err(Error::RequestRejected(res)) => {
                    log::error!("Failed to spot subscribe the symbol_id {:?}", symbol_id);
                    self.spot_req_states.lock().await.remove(&symbol_id);
                    return Err(Error::SubscriptionError(
                        symbol_id,
                        res.get_field_value(Field::Text)
                            .expect("No Text tag in Reject response"),
                        MarketType::Spot,
                    ));
                }
                _ => {
                    // no response
                    // retrigger
                    if let Err(err) = self.internal.trigger.send(()).await {
                        return Err(Error::TriggerError(err));
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn unsubscribe_spot(&mut self, symbol_id: u32) -> Result<(), Error> {
        // if let Some(RequestState::Requested) =
        let states = self
            .spot_req_states
            .lock()
            .await
            .get(&symbol_id)
            .map(|v| *v);

        match states {
            Some(RequestState::Requested) => {
                return Err(Error::RequestingSubscription(symbol_id, MarketType::Spot));
            }
            None => {
                return Err(Error::NotSubscribed(symbol_id, MarketType::Spot));
            }
            _ => {
                self.spot_req_states.lock().await.remove(&symbol_id);
                self.spot_market_data.lock().await.remove(&symbol_id);
                let req = MarketDataReq::new("-1".into(), '2', 1, None, &['0', '1'], 1, symbol_id);
                let _seq_num = self.internal.send_message(req).await?;

                log::trace!("Unsubscribed spot for symbol({})", symbol_id);

                // no need to wait
                Ok(())
            }
        }
    }

    pub async fn subscribe_depth(&mut self, symbol_id: u32) -> Result<(), Error> {
        // check already subscribed?
        // FIXME : two states - requested accepted
        if self.depth_req_states.lock().await.contains_key(&symbol_id) {
            return Err(Error::SubscribedAlready(symbol_id, MarketType::Depth));
        }

        // add to requested symbol.
        self.depth_req_states
            .lock()
            .await
            .insert(symbol_id, RequestState::Requested);

        // intialize the request and send req
        let req = MarketDataReq::new("-1".into(), '1', 0, None, &['0', '1'], 1, symbol_id);
        let seq_num = self.internal.send_message(req).await?;

        // waiting (reject or  marketdata)
        while let Ok(()) = self.internal.wait_notifier().await {
            // check the market data
            if let Some(RequestState::Accepted) = self.depth_req_states.lock().await.get(&symbol_id)
            {
                // accepted
                log::trace!("Depth data subscription accepted for symbol({})", symbol_id);
                break;
            }

            match self.internal.check_req_accepted(seq_num).await {
                Err(Error::RequestRejected(res)) => {
                    log::error!("Failed to depth subscribe the symbol_id {:?}", symbol_id);
                    self.depth_req_states.lock().await.remove(&symbol_id);
                    return Err(Error::SubscriptionError(
                        symbol_id,
                        res.get_field_value(Field::Text)
                            .expect("No Text tag in Reject response"),
                        MarketType::Depth,
                    ));
                }
                _ => {
                    // no response
                    // retrigger
                    if let Err(err) = self.internal.trigger.send(()).await {
                        return Err(Error::TriggerError(err));
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn unsubscribe_depth(&mut self, symbol_id: u32) -> Result<(), Error> {
        let states = self
            .depth_req_states
            .lock()
            .await
            .get(&symbol_id)
            .map(|v| *v);

        match states {
            Some(RequestState::Requested) => {
                return Err(Error::RequestingSubscription(symbol_id, MarketType::Depth));
            }
            None => {
                return Err(Error::NotSubscribed(symbol_id, MarketType::Depth));
            }
            _ => {
                self.depth_req_states.lock().await.remove(&symbol_id);
                self.depth_market_data.lock().await.remove(&symbol_id);
                let req = MarketDataReq::new("-1".into(), '2', 0, None, &['0', '1'], 1, symbol_id);
                let _seq_num = self.internal.send_message(req).await?;

                log::trace!("Unsubscribed depth for symbol({})", symbol_id);

                Ok(())
            }
        }
    }
}
