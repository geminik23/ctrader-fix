mod models;
use std::collections::VecDeque;
use std::sync::Arc;

use async_std::io::{BufReader, BufWriter};
use async_std::sync::Mutex;
use models::{Config, HeartbeatReq, LogonReq, RequestMessage, SubID};
use models::{LogoutReq, ResponseMessage};

use async_std::net::TcpStream;
use async_std::prelude::*;
use async_std::task;

pub struct Socket {
    stream: Arc<TcpStream>,
    msg_buffer: String,
    delimiter: String,
}

pub struct FixApi {
    config: Config,
    quote: StreamData,
    trade: StreamData,
    delimiter: String,
}

pub struct StreamData {
    stream: Option<Arc<TcpStream>>,
    seq: u32,
}

impl FixApi {
    pub fn new(
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
            quote: StreamData {
                stream: None,
                seq: 1,
            },
            trade: StreamData {
                stream: None,
                seq: 1,
            },
            delimiter: "\u{1}".into(),
        }
    }

    pub async fn disconnect(&mut self) -> std::io::Result<()> {
        if let Some(stream) = &mut self.quote.stream {
            stream.shutdown(std::net::Shutdown::Both)?;
        }
        if let Some(stream) = &mut self.trade.stream {
            stream.shutdown(std::net::Shutdown::Both)?;
        }

        self.quote.stream = None;
        self.trade.stream = None;

        Ok(())
    }

    pub async fn connect(&mut self) -> std::io::Result<()> {
        let mut qsocket = Socket::connect(self.config.host.as_str(), 5201, &self.delimiter).await?;
        let mut tsocket = Socket::connect(self.config.host.as_str(), 5202, &self.delimiter).await?;

        self.quote.stream = Some(qsocket.stream.clone());
        self.trade.stream = Some(tsocket.stream.clone());

        let _qhandle = {
            task::spawn(async move {
                qsocket.recv_loop().await.unwrap();
            })
        };

        let _thandle = {
            task::spawn(async move {
                tsocket.recv_loop().await.unwrap();
            })
        };

        Ok(())
    }

    async fn send_message<R: RequestMessage>(
        &mut self,
        sub_id: SubID,
        req: R,
    ) -> std::io::Result<()> {
        match sub_id {
            SubID::QUOTE => {
                let req = req.build(sub_id, self.quote.seq, &self.delimiter, &self.config);
                println!("{:?}", req);
                self.quote.seq += 1;
                if let Some(qstream) = self.quote.stream.as_mut() {
                    let mut writer = BufWriter::new(qstream.as_ref());
                    writer.write_all(req.as_bytes()).await?;
                    writer.flush().await?;
                }
                // TODO failed? then close?
            }
            SubID::TRADE => {
                let req = req.build(sub_id, self.trade.seq, &self.delimiter, &self.config);
                self.trade.seq += 1;
                if let Some(tstream) = self.trade.stream.as_mut() {
                    let mut writer = BufWriter::new(tstream.as_ref());
                    writer.write_all(req.as_bytes()).await?;
                    writer.flush().await?;
                }
                // TODO failed? then close?
            }
        }

        Ok(())
    }

    //
    // handle messages
    //

    pub fn handle_message(&self, res: ResponseMessage) {
        // match res.get_message_type() {
        //     _ => {}
        // }
    }

    //
    // request
    //

    pub async fn heartbeat_quote(&mut self) -> std::io::Result<()> {
        self.send_message(SubID::QUOTE, HeartbeatReq::default())
            .await?;
        Ok(())
    }

    pub async fn heartbeat_trade(&mut self) -> std::io::Result<()> {
        self.send_message(SubID::TRADE, HeartbeatReq::default())
            .await?;
        Ok(())
    }

    pub async fn logon(&mut self) -> std::io::Result<()> {
        self.send_message(SubID::QUOTE, LogonReq::default()).await?;
        self.send_message(SubID::TRADE, LogonReq::default()).await?;
        Ok(())
    }

    pub async fn logout(&mut self) -> std::io::Result<()> {
        self.send_message(SubID::QUOTE, LogoutReq::default())
            .await?;
        self.send_message(SubID::TRADE, LogoutReq::default())
            .await?;
        Ok(())
    }
}

impl Socket {
    pub async fn connect(server: &str, port: u16, delimiter: &str) -> std::io::Result<Self> {
        let addr = format!("{}:{}", server, port);
        let stream = TcpStream::connect(addr).await?;
        Ok(Socket {
            stream: Arc::new(stream),
            // res_msg: Arc::new(Mutex::new(VecDeque::new())),
            msg_buffer: String::new(),
            delimiter: delimiter.into(),
        })
    }

    pub async fn recv_loop(&mut self) -> std::io::Result<()> {
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

            if self
                .msg_buffer
                .find(&format!("{}10=", self.delimiter))
                .is_some()
                && self.msg_buffer.ends_with(&self.delimiter)
            {
                let res = ResponseMessage::new(&self.msg_buffer, &self.delimiter);

                // TODO check the heartbeat?

                self.msg_buffer.clear();
            }
        }
        Ok(())
    }
}
