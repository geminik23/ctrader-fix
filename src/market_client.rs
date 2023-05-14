use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use async_std::sync::{Mutex, RwLock};
use async_std::task;

use crate::{
    fixapi::FixApi,
    messages::MarketDataReq,
    types::{
        ConnectionHandler, DepthPrice, Error, Field, IncrementalRefresh, MarketDataHandler,
        MarketType, SpotPrice,
    },
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

    spot_market_data: Arc<Mutex<HashMap<u32, SpotPrice>>>,
    depth_market_data: Arc<RwLock<HashMap<u32, HashMap<String, DepthPrice>>>>,
    //
    //
    market_data_handler: Option<Arc<dyn MarketDataHandler + Send + Sync>>,
}

fn insert_entry_to(e: HashMap<Field, String>, depth_data: &mut HashMap<String, DepthPrice>) {
    if e.len() < 4 {
        return;
    }
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

fn depth_data_from_entries(data: Vec<HashMap<Field, String>>) -> HashMap<String, DepthPrice> {
    let mut depth_data = HashMap::new();
    for e in data.into_iter() {
        insert_entry_to(e, &mut depth_data);
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
        Self {
            internal: FixApi::new(
                crate::types::SubID::QUOTE,
                host,
                login,
                password,
                broker,
                heartbeat_interval,
            ),

            spot_req_states: Arc::new(Mutex::new(HashMap::new())),
            spot_market_data: Arc::new(Mutex::new(HashMap::new())),

            depth_req_states: Arc::new(Mutex::new(HashMap::new())),
            depth_market_data: Arc::new(RwLock::new(HashMap::new())),
            market_data_handler: None,
        }
    }
    pub fn register_market_handler_arc<T: MarketDataHandler + Send + Sync + 'static>(
        &mut self,
        handler: Arc<T>,
    ) {
        self.market_data_handler = Some(handler);
    }

    pub fn register_market_handler<T: MarketDataHandler + Send + Sync + 'static>(
        &mut self,
        handler: T,
    ) {
        self.market_data_handler = Some(Arc::new(handler));
    }

    pub fn register_connection_handler_arc<T: ConnectionHandler + Send + Sync + 'static>(
        &mut self,
        handler: Arc<T>,
    ) {
        self.internal.register_connection_handler_arc(handler);
    }

    pub fn register_connection_handler<T: ConnectionHandler + Send + Sync + 'static>(
        &mut self,
        handler: T,
    ) {
        self.internal.register_connection_handler(handler);
    }

    fn register_internal_handler(&mut self) {
        // clone
        let trigger = self.internal.trigger.clone();
        let spot_req_states_clone = self.spot_req_states.clone();
        let spot_market_data_clone = self.spot_market_data.clone();

        let depth_req_states_clone = self.depth_req_states.clone();
        let depth_market_data_clone = self.depth_market_data.clone();

        let market_data_handler = self.market_data_handler.clone();

        let market_callback =
            move |msg_type: char, symbol_id: u32, data: Vec<HashMap<Field, String>>| {
                // symbol_id is only valid for msg_type - 'W'

                let tx = trigger.clone();
                let spot_req_states_clone = spot_req_states_clone.clone();
                let spot_market_data_clone = spot_market_data_clone.clone();
                let depth_req_states_clone = depth_req_states_clone.clone();
                let depth_market_data_clone = depth_market_data_clone.clone();

                let market_data_handler = market_data_handler.clone();

                //
                task::spawn(async move {
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
                                        .insert(symbol_id, prices.clone());

                                    // to handler
                                    if let Some(handler) = market_data_handler {
                                        handler.on_price_of(symbol_id, prices).await;
                                    }
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
                                    let depth_data = depth_data_from_entries(data);

                                    // FIXME which one should be first?
                                    // to handler
                                    if let Some(handler) = market_data_handler {
                                        handler
                                            .on_market_depth_full_refresh(
                                                symbol_id,
                                                depth_data.clone(),
                                            )
                                            .await;
                                    }

                                    // update the depth data
                                    depth_market_data_clone
                                        .write()
                                        .await
                                        .insert(symbol_id, depth_data);
                                }
                            }
                        }
                        'X' => {
                            // ignore the symbol_id argument
                            //
                            // Market data incremental refresh
                            if Some(&RequestState::Accepted)
                                == depth_req_states_clone.lock().await.get(&symbol_id)
                            {
                                let mut incre_list = Vec::new();
                                for e in data.into_iter() {
                                    let symbol =
                                        e.get(&Field::Symbol).unwrap().parse::<u32>().unwrap();

                                    match e.get(&Field::MDUpdateAction) {
                                        Some(s) if s == "2" => {
                                            // delete
                                            incre_list.push(IncrementalRefresh::Delete {
                                                symbol_id: symbol,
                                                entry_id: e.get(&Field::MDEntryID).unwrap().clone(),
                                            });
                                        }
                                        Some(s) if s == "0" => {
                                            // new
                                            let eid = e.get(&Field::MDEntryID).unwrap();
                                            incre_list.push(IncrementalRefresh::New {
                                                symbol_id: symbol,
                                                entry_id: eid.clone(),
                                                data: DepthPrice {
                                                    price_type: e
                                                        .get(&Field::MDEntryType)
                                                        .unwrap()
                                                        .parse()
                                                        .unwrap(),
                                                    price: e
                                                        .get(&Field::MDEntryPx)
                                                        .unwrap()
                                                        .parse::<f64>()
                                                        .unwrap(),
                                                    size: e
                                                        .get(&Field::MDEntrySize)
                                                        .unwrap()
                                                        .parse::<f64>()
                                                        .unwrap(),
                                                },
                                            });
                                        }
                                        _ => {}
                                    }
                                }

                                // FIXME which one should be first?
                                // to handler
                                if let Some(handler) = market_data_handler {
                                    handler
                                        .on_market_depth_incremental_refresh(incre_list.clone())
                                        .await;
                                }

                                {
                                    let mut depth_cont = depth_market_data_clone.write().await;
                                    for incre in incre_list.into_iter() {
                                        match incre {
                                            IncrementalRefresh::New {
                                                symbol_id,
                                                entry_id,
                                                data,
                                            } => {
                                                let s = depth_cont
                                                    .entry(symbol_id)
                                                    .or_insert(HashMap::new());
                                                s.insert(entry_id, data);
                                            }
                                            IncrementalRefresh::Delete {
                                                symbol_id,
                                                entry_id,
                                            } => {
                                                let s = depth_cont
                                                    .entry(symbol_id)
                                                    .or_insert(HashMap::new());
                                                s.remove(&entry_id);
                                            }
                                        }
                                    }
                                }
                                //
                            }
                        }
                        _ => {}
                    }
                });
            };
        self.internal.register_market_callback(market_callback);
    }

    /// Connects to a server
    ///
    /// This method first attempt to establish a connection. If the connection is succesful, then
    /// it proceeds to logon directly.
    pub async fn connect(&mut self) -> Result<(), Error> {
        // set market handler
        self.register_internal_handler();

        // connection
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
            .read()
            .await
            .get(&symbol_id)
            .map(|v| v.clone())
            .ok_or(Error::NotSubscribed(symbol_id, MarketType::Spot))
    }

    pub async fn subscribe_spot(&self, symbol_id: u32) -> Result<(), Error> {
        // FIXME later
        // .. code is too messy. is there a better way?

        // check already subscribed?
        if let Some(state) = self.spot_req_states.lock().await.get(&symbol_id) {
            match state {
                RequestState::Accepted => {
                    return Err(Error::SubscribedAlready(symbol_id, MarketType::Spot));
                }
                RequestState::Requested => {
                    return Err(Error::RequestingSubscription(symbol_id, MarketType::Spot));
                }
            }
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
                    // FIXME is it necessary?
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

    pub async fn unsubscribe_spot(&self, symbol_id: u32) -> Result<(), Error> {
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

    pub async fn subscribe_depth(&self, symbol_id: u32) -> Result<(), Error> {
        // check already subscribed?
        if let Some(state) = self.depth_req_states.lock().await.get(&symbol_id) {
            match state {
                RequestState::Accepted => {
                    return Err(Error::SubscribedAlready(symbol_id, MarketType::Depth));
                }
                RequestState::Requested => {
                    return Err(Error::RequestingSubscription(symbol_id, MarketType::Depth));
                }
            }
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

    pub async fn unsubscribe_depth(&self, symbol_id: u32) -> Result<(), Error> {
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
                self.depth_market_data.write().await.remove(&symbol_id);
                let req = MarketDataReq::new("-1".into(), '2', 0, None, &['0', '1'], 1, symbol_id);
                let _seq_num = self.internal.send_message(req).await?;

                log::trace!("Unsubscribed depth for symbol({})", symbol_id);

                Ok(())
            }
        }
    }
}
