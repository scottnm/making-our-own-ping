use crate::iphlp;
use crate::ipv4;

pub struct Request {
    msg: String,
    ttl: u8,
    timeout: u32,
}

#[derive(Debug)]
pub struct Reply<'a> {
    // field should never be referenced directly since
    // it's just a backing buffer for icmpr
    _buffer: Vec<u8>,
    data: &'a [u8],
    ttl: std::time::Duration,
    addr: ipv4::Addr,
    rtt: std::time::Duration,
}

impl Reply<'_> {
    pub fn data(&self) -> &[u8] {
        self.data
    }

    pub fn rtt(&self) -> std::time::Duration {
        self.rtt
    }

    pub fn ttl(&self) -> std::time::Duration {
        self.ttl
    }
}

impl Request {
    pub fn new() -> Self {
        Self {
            msg: String::from("Hello! Anybody there?"),
            ttl: 128,
            timeout: 4000,
        }
    }
    pub fn msg<S>(mut self, request: S) -> Self
    where
        S: AsRef<str>,
    {
        self.msg = String::from(request.as_ref());
        self
    }

    pub fn ttl(mut self, ttl: u8) -> Self {
        self.ttl = ttl;
        self
    }

    pub fn timeout(mut self, timeout: u32) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn send(self, addr: &ipv4::Addr) -> Result<Reply, String> {
        let mut reply_buffer =
            vec![0u8; std::mem::size_of::<iphlp::IcmpEchoReply>() + 8 + self.msg.len()];

        let echo_options = iphlp::IpOptionInformation {
            ttl: self.ttl,
            tos: 0,
            flags: 0,
            options_data32: 0,
            options_size: 0,
        };

        // FIXME: RAII-ify
        let echo_result = {
            let icmp_file = iphlp::IcmpCreateFile();

            let echo_result = iphlp::IcmpSendEcho(
                icmp_file,
                *addr,
                self.msg.as_ptr(),
                self.msg.len() as u16,
                Some(&echo_options),
                reply_buffer.as_mut_ptr(),
                reply_buffer.len() as u32,
                self.timeout,
            );

            iphlp::IcmpCloseHandle(icmp_file);

            echo_result
        };

        match echo_result {
            0 => Err(String::from("IcmpSendEcho failed! No replies")),
            1 => {
                let reply_ref =
                    unsafe { std::mem::transmute::<&u8, &iphlp::IcmpEchoReply>(&reply_buffer[0]) };
                Ok(Reply {
                    _buffer: reply_buffer,
                    data: reply_ref.data(),
                    ttl: std::time::Duration::from_secs(reply_ref.options.ttl as u64),
                    addr: reply_ref.addr,
                    rtt: std::time::Duration::from_millis(reply_ref.rtt as u64),
                })
            }
            _ => Err(std::format!("Unexpected reply count! {}", echo_result)),
        }
    }
}
