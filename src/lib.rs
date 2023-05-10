pub mod models;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_std::channel::{bounded, Receiver, Sender};
use async_std::io::{BufReader, BufWriter};
use async_std::net::TcpStream;
use async_std::prelude::*;
use async_std::stream::{self, StreamExt};
use async_std::sync::RwLock;
use async_std::task;
use models::{Config, Field, HeartbeatReq, LogonReq, RequestMessage, SubID, TestReq};
use models::{LogoutReq, ResponseMessage};

pub struct Socket {
    stream: Arc<TcpStream>,
    res_sender: Sender<ResponseMessage>,
    msg_buffer: String,
}

const DELIMITER: &str = "\u{1}";

pub struct BaseFixApi {
    config: Config,
    stream: Option<Arc<TcpStream>>,
    seq: Arc<AtomicU32>,
    sub_id: SubID,

    is_connected: Arc<AtomicBool>,

    res_receiver: Option<Receiver<ResponseMessage>>,
    notifier: Option<Receiver<()>>,

    container: Arc<RwLock<Vec<ResponseMessage>>>,
}

pub struct StreamData {}

impl BaseFixApi {
    pub fn new(
        sub_id: SubID,
        host: String,
        login: String,
        password: String,
        broker: String,
        heartbeat_interval: Option<u32>,
    ) -> Self {
        Self {
            config: Config::new(
                host,
                login,
                password,
                broker,
                heartbeat_interval.unwrap_or(30),
            ),
            stream: None,
            res_receiver: None,
            notifier: None,
            is_connected: Arc::new(AtomicBool::new(false)),
            seq: Arc::new(AtomicU32::new(1)),
            container: Arc::new(RwLock::new(Vec::new())),
            sub_id,
        }
    }

    pub async fn disconnect(&mut self) -> std::io::Result<()> {
        if let Some(stream) = &mut self.stream {
            stream.shutdown(std::net::Shutdown::Both)?;
        }
        self.stream = None;
        self.res_receiver = None;
        self.notifier = None;
        self.is_connected.store(false, Ordering::Release);
        Ok(())
    }

    pub async fn connect(&mut self) -> std::io::Result<()> {
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
        self.res_receiver = Some(receiver);
        self.stream = Some(socket.stream.clone());

        let is_connected = self.is_connected.clone();

        let _handle = task::spawn(async move {
            socket.recv_loop(is_connected).await.unwrap();
        });

        Ok(())
    }

    pub async fn send_message<R: RequestMessage>(&mut self, req: R) -> std::io::Result<()> {
        let req = req.build(
            self.sub_id,
            self.seq.fetch_add(1, Ordering::Relaxed),
            DELIMITER,
            &self.config,
        );
        if let Some(stream) = self.stream.as_mut() {
            let mut writer = BufWriter::new(stream.as_ref());
            writer.write_all(req.as_bytes()).await?;
            writer.flush().await?;
        }

        // TODO wait

        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::Relaxed)
    }

    //
    // request
    //
    pub async fn heartbeat(&mut self) -> std::io::Result<()> {
        self.send_message(HeartbeatReq::default()).await?;
        Ok(())
    }

    pub async fn logon(&mut self) -> std::io::Result<()> {
        // TODO check the connected

        self.send_message(LogonReq::default()).await?;

        // wait to receive the response
        if let Some(recv) = &self.res_receiver {
            while let Ok(response) = recv.recv().await {
                // logon response
                if response.get_message_type() == "A" {
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
                    let (tx, rx) = bounded(1);
                    self.notifier = Some(rx);
                    let recv = self.res_receiver.clone().unwrap();
                    let cont = self.container.clone();

                    task::spawn(async move {
                        while let Ok(res) = recv.recv().await {
                            // notify? or send? via channel?
                            match res.get_message_type() {
                                "1" => {
                                    // send back with test request id
                                    if let Some(test_req_id) = res.get_field_value(Field::TestReqID)
                                    {
                                        send_request_clone(Box::new(TestReq::new(test_req_id)))
                                            .await;
                                        log::debug!("Sent the heartbeat from test_req_id");
                                    }
                                }
                                // "3" => process_reject,
                                // "5" => process_logout,
                                // "8" => process_exec_report,
                                // "9" => process_reject,
                                // "A" => process_logon,
                                // "j" => process_reject,
                                // "W" => process_market_data,
                                // "X" => process_market_incr_data,
                                // "y" => process_sec_list,
                                // "AP" => process_position_list,
                                _ => {
                                    // store the response in container.
                                    let mut cont = cont.write().await;
                                    cont.push(res);
                                    tx.send(()).await.unwrap_or_else(|e| {
                                        log::error!(
                                            "Failed to notify that the response is received - {:?}",
                                            e
                                        );
                                    });
                                }
                            }
                        }
                    });

                    break;
                }
            }
        }

        Ok(())
    }

    pub async fn logout(&mut self) -> std::io::Result<()> {
        self.send_message(LogoutReq::default()).await?;
        Ok(())
    }
}

impl Socket {
    pub async fn connect(
        server: &str,
        port: u16,
        res_sender: Sender<ResponseMessage>,
    ) -> std::io::Result<Self> {
        let addr = format!("{}:{}", server, port);
        let stream = TcpStream::connect(addr).await?;
        Ok(Socket {
            stream: Arc::new(stream),
            res_sender,
            // res_msg: Arc::new(Mutex::new(VecDeque::new())),
            msg_buffer: String::new(),
        })
    }

    pub async fn recv_loop(&mut self, is_connected: Arc<AtomicBool>) -> std::io::Result<()> {
        let mut reader = BufReader::new(self.stream.as_ref());
        let mut buffer = vec![0u8; 4096];
        loop {
            let bytes_read = reader.read(&mut buffer).await?;
            if bytes_read == 0 {
                log::error!("Disconnected");
                break;
            }
            let res = String::from_utf8_lossy(&buffer[..bytes_read]);
            log::trace!("Received msg : {}", res);
            self.msg_buffer.push_str(res.as_ref());
            // println!("{:?}", self.msg_buffer);

            if self.msg_buffer.find(&format!("{}10=", DELIMITER)).is_some()
                && self.msg_buffer.ends_with(DELIMITER)
            {
                log::debug!("Handle response : {}", self.msg_buffer);
                let res = ResponseMessage::new(&self.msg_buffer, DELIMITER);
                if let Err(err) = self.res_sender.send(res).await {
                    log::error!("Failed to send ResponseMessage : {:?}", err);
                    break;
                }
                self.msg_buffer.clear();
            }
        }
        is_connected.store(false, Ordering::Relaxed);
        Ok(())
    }
}
