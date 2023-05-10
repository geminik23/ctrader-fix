use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use async_std::{
    channel::Sender,
    io::{BufReader, ReadExt},
    net::TcpStream,
};

use crate::{messages::ResponseMessage, types::DELIMITER};

pub struct Socket {
    pub stream: Arc<TcpStream>,
    res_sender: Sender<ResponseMessage>,
    msg_buffer: String,
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
