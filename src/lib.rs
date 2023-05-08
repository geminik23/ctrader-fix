mod models;
use std::sync::Arc;

use async_std::io::{BufReader, BufWriter};
use models::ResponseMessage;
use models::{Config, HeartbeatReq, LogonReq, RequestMessage, SubID};

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
    qstream: Option<Arc<TcpStream>>,
    qseq: u32,
    tstream: Option<Arc<TcpStream>>,
    tseq: u32,

    delimiter: String,
}

impl FixApi {
    pub fn new(host: String, login: String, password: String, broker: String) -> Self {
        Self {
            config: Config::new(host, login, password, broker, 30),
            qstream: None,
            tstream: None,
            qseq: 1,
            tseq: 1,
            delimiter: "\u{1}".into(),
        }
    }

    pub async fn disconnect(&mut self) -> std::io::Result<()> {
        if let Some(stream) = &mut self.qstream {
            stream.shutdown(std::net::Shutdown::Both)?;
        }
        if let Some(stream) = &mut self.tstream {
            stream.shutdown(std::net::Shutdown::Both)?;
        }

        self.qstream = None;
        self.tstream = None;

        Ok(())
    }

    pub async fn connect(&mut self) -> std::io::Result<()> {
        let mut qsocket = Socket::connect(self.config.host.as_str(), 5201, &self.delimiter).await?;
        let mut tsocket = Socket::connect(self.config.host.as_str(), 5202, &self.delimiter).await?;

        self.qstream = Some(qsocket.stream.clone());
        self.tstream = Some(tsocket.stream.clone());

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
                let req = req.build(sub_id, self.qseq, &self.delimiter, &self.config);
                println!("{:?}", req);
                self.qseq += 1;
                if let Some(qstream) = self.qstream.as_mut() {
                    let mut writer = BufWriter::new(qstream.as_ref());
                    writer.write_all(req.as_bytes()).await?;
                    writer.flush().await?;
                }
                // TODO failed? then close?
            }
            SubID::TRADE => {
                let req = req.build(sub_id, self.tseq, &self.delimiter, &self.config);
                self.tseq += 1;
                if let Some(tstream) = self.tstream.as_mut() {
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
}

impl Socket {
    pub async fn connect(server: &str, port: u16, delimiter: &str) -> std::io::Result<Self> {
        let addr = format!("{}:{}", server, port);
        let stream = TcpStream::connect(addr).await?;
        Ok(Socket {
            stream: Arc::new(stream),
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
