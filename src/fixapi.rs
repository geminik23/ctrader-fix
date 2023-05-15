use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};

use async_std::{
    channel::{bounded, Receiver, Sender},
    io::{BufWriter, WriteExt},
    net::TcpStream,
    stream::{self, StreamExt},
    sync::RwLock,
    task,
};

use crate::types::{Config, Error, Field, InternalMDResult, SubID, DELIMITER};
use crate::{
    messages::{HeartbeatReq, LogonReq, LogoutReq, RequestMessage, ResponseMessage, TestReq},
    types::ConnectionHandler,
};
use crate::{socket::Socket, types::MarketCallback};

pub struct FixApi {
    config: Config,
    stream: Option<Arc<TcpStream>>,
    seq: Arc<AtomicU32>,
    sub_id: SubID,

    is_connected: Arc<AtomicBool>,

    res_receiver: Option<Receiver<ResponseMessage>>,
    pub trigger: Sender<String>,
    listener: Receiver<String>,

    pub container: Arc<RwLock<HashMap<u32, ResponseMessage>>>,

    //callback
    connection_handler: Option<Arc<dyn ConnectionHandler + Send + Sync>>,
    market_callback: Option<MarketCallback>,
}

impl FixApi {
    pub fn new(
        sub_id: SubID,
        host: String,
        login: String,
        password: String,
        sender_comp_id: String,
        heartbeat_interval: Option<u32>,
    ) -> Self {
        let (tx, rx) = bounded(1);
        Self {
            config: Config::new(
                host,
                login,
                password,
                sender_comp_id,
                heartbeat_interval.unwrap_or(30),
            ),
            stream: None,
            res_receiver: None,
            trigger: tx,
            listener: rx,
            is_connected: Arc::new(AtomicBool::new(false)),
            seq: Arc::new(AtomicU32::new(1)),
            container: Arc::new(RwLock::new(HashMap::new())),
            sub_id,
            connection_handler: None,
            market_callback: None,
        }
    }

    pub fn register_market_callback<F>(&mut self, callback: F)
    where
        F: Fn(InternalMDResult) -> () + Send + Sync + 'static,
    {
        self.market_callback = Some(Arc::new(move |mdresult: InternalMDResult| -> () {
            callback(mdresult)
        }));
    }

    pub fn register_connection_handler_arc<T: ConnectionHandler + Send + Sync + 'static>(
        &mut self,
        handler: Arc<T>,
    ) {
        self.connection_handler = Some(handler);
    }

    pub fn register_connection_handler<T: ConnectionHandler + Send + Sync + 'static>(
        &mut self,
        handler: T,
    ) {
        self.connection_handler = Some(Arc::new(handler));
    }

    pub async fn disconnect(&mut self) -> Result<(), Error> {
        if let Some(stream) = self.stream.clone() {
            stream.shutdown(std::net::Shutdown::Both)?;
        }
        self.stream = None;
        self.res_receiver = None;
        self.is_connected.store(false, Ordering::Relaxed);
        Ok(())
    }

    pub async fn connect(&mut self) -> Result<(), Error> {
        let (sender, receiver) = bounded(1);
        let mut socket = Socket::connect(
            self.config.host.as_str(),
            if self.sub_id == SubID::QUOTE {
                5201
            } else {
                5202
            },
            sender,
        )
        .await?;
        self.is_connected.store(true, Ordering::Relaxed);
        log::debug!("stream connected");

        // notify connection
        if let Some(handler) = self.connection_handler.clone() {
            task::spawn(async move {
                handler.on_connect().await;
            });
        }

        self.res_receiver = Some(receiver);
        self.stream = Some(socket.stream.clone());

        let is_connected = self.is_connected.clone();

        let handler = self.connection_handler.clone();
        let _ = task::spawn(async move {
            socket.recv_loop(is_connected, handler).await.unwrap();
        });

        Ok(())
    }

    pub async fn send_message<R: RequestMessage>(&self, req: R) -> Result<u32, Error> {
        let no_seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let req = req.build(self.sub_id, no_seq, DELIMITER, &self.config);
        if let Some(stream) = self.stream.clone() {
            log::debug!("Send request : {}", req);
            let mut writer = BufWriter::new(stream.as_ref());
            writer.write_all(req.as_bytes()).await?;
            writer.flush().await?;
        }

        Ok(no_seq)
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::Relaxed)
    }

    pub async fn wait_notifier(&self) -> Result<String, Error> {
        if !self.is_connected() {
            return Err(Error::NotConnected);
        }
        self.listener.recv().await.map_err(|e| e.into())
    }

    pub async fn check_req_accepted(&self, seq_num: u32) -> Result<ResponseMessage, Error> {
        let mut cont = self.container.write().await;
        //check the seq no
        if let Some(res) = cont.remove(&seq_num) {
            // rejected
            let msg_type = res.get_message_type();
            if msg_type == "3" || msg_type == "j" {
                log::debug!("Got rejected");
                return Err(Error::RequestRejected(res));
            }
            Ok(res)
        } else {
            Err(Error::NoResponse(seq_num))
        }
    }
    //
    // request
    //
    // pub async fn heartbeat(&mut self) -> Result<(), Error> {
    //     self.send_message(HeartbeatReq::default()).await?;
    //     Ok(())
    // }

    pub async fn logon(&self) -> Result<(), Error> {
        // TODO check the connected

        self.send_message(LogonReq::default()).await?;

        // wait to receive the response
        if let Some(recv) = &self.res_receiver {
            while let Ok(response) = recv.recv().await {
                // logon response
                let msg_type = response.get_message_type();
                match msg_type {
                    "A" => {
                        //
                        if let Some(handler) = self.connection_handler.clone() {
                            task::spawn(async move {
                                handler.on_logon().await;
                            });
                        }

                        let stream = self.stream.clone().unwrap();
                        let sub_id = self.sub_id;
                        let config = self.config.clone();
                        let seq = self.seq.clone();

                        let send_request = move |req: Box<dyn RequestMessage>| {
                            let stream = stream.clone();
                            let sub_id = sub_id;
                            let config = config.clone();
                            let seq = seq.clone();
                            async move {
                                let req = req.build(
                                    sub_id,
                                    seq.fetch_add(1, Ordering::Relaxed),
                                    DELIMITER,
                                    &config,
                                );

                                let mut writer = BufWriter::new(stream.as_ref());
                                let _ = writer.write_all(req.as_bytes()).await;
                                writer.flush().await.unwrap_or_else(|e| {
                                    log::error!("Failed to send the heartbeat request - {:?}", e);
                                });
                            }
                        };
                        let send_request_clone = send_request.clone();

                        let hb_interval = self.config.heart_beat as u64;

                        //
                        // send heartbeat per hb_interval
                        task::spawn(async move {
                            let mut heartbeat_stream =
                                stream::interval(Duration::from_secs(hb_interval));

                            while let Some(_) = heartbeat_stream.next().await {
                                let req = HeartbeatReq::default();
                                send_request(Box::new(req)).await;
                                log::debug!("Sent the heartbeat");
                            }
                        });

                        //
                        // handle the responses

                        // notifier
                        let tx = self.trigger.clone();
                        let recv = self.res_receiver.clone().unwrap();
                        let cont = self.container.clone();
                        let market_callback = self.market_callback.clone();

                        task::spawn(async move {
                            while let Ok(res) = recv.recv().await {
                                let msg_type = res.get_message_type();
                                let mtype = String::from(msg_type);
                                // notify? or send? via channel?
                                match msg_type {
                                    "5" => {
                                        // 5 : logout
                                        tx.send(mtype).await.unwrap_or_else(|e| {
                                            // fatal
                                            log::error!(
                                            "Failed to notify that the response is received - {:?}",
                                            e
                                            );
                                        });
                                    }
                                    "1" => {
                                        // send back with test request id
                                        if let Some(test_req_id) =
                                            res.get_field_value(Field::TestReqID)
                                        {
                                            send_request_clone(Box::new(TestReq::new(test_req_id)))
                                                .await;
                                            log::debug!("Sent the heartbeat from test_req_id");
                                        }
                                    }
                                    "W" | "X" | "Y" => {
                                        // market data
                                        let symbol_id = res
                                            .get_field_value(Field::Symbol)
                                            .unwrap_or("0".into())
                                            .parse::<u32>()
                                            .unwrap();
                                        // notify to callback
                                        if let Some(market_callback) = market_callback.clone() {
                                            let mdresult = if msg_type == "Y" {
                                                let md_req_id = res
                                                    .get_field_value(Field::MDReqID)
                                                    .map(|v| v.clone())
                                                    .unwrap_or("".into());
                                                let err_msg = res
                                                    .get_field_value(Field::Text)
                                                    .map(|v| v.clone())
                                                    .unwrap_or("".into());
                                                InternalMDResult::MDReject {
                                                    symbol_id,
                                                    md_req_id,
                                                    err_msg,
                                                }
                                            } else {
                                                let data = res.get_repeating_groups(
                                                    Field::NoMDEntries,
                                                    if msg_type == "W" {
                                                        Field::MDEntryType
                                                    } else {
                                                        Field::MDUpdateAction
                                                    },
                                                    None,
                                                );
                                                InternalMDResult::MD {
                                                    msg_type: msg_type.chars().next().unwrap(),
                                                    symbol_id,
                                                    data,
                                                }
                                            };

                                            market_callback(mdresult);
                                        }
                                    }
                                    _ => {
                                        log::debug!("{}", res.get_message());

                                        if let Some(seq_num) = res.get_field_value(Field::MsgSeqNum)
                                        {
                                            // store the response in container.
                                            {
                                                let mut cont = cont.write().await;
                                                cont.insert(
                                                    seq_num.parse().expect(
                                                        "Failed to parse the MsgSeqNum into u32",
                                                    ),
                                                    res,
                                                );
                                            }
                                            tx.send(mtype).await.unwrap_or_else(|e| {
                                                // fatal
                                                log::error!(
                                                "Failed to notify that the response is received - {:?}",
                                                e
                                            );
                                        });
                                        } else {
                                            log::debug!("No seq number : {}", res.get_message());
                                        }
                                    }
                                }
                            }
                        });

                        break;
                    }
                    "5" => {
                        return Err(Error::LoggedOut);
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    pub async fn logout(&self) -> Result<(), Error> {
        self.send_message(LogoutReq::default()).await?;
        Ok(())
    }
}
