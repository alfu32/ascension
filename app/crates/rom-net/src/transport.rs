//! WebSocket plumbing shared by the server and the client. One thread per connection reads
//! with a short timeout and, between reads, writes whatever is queued for it, so a blocking
//! socket serves both directions without an async runtime.

use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

use tungstenite::protocol::WebSocketConfig;
use tungstenite::protocol::frame::CloseFrame;
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::{Message, WebSocket};

use crate::NetError;
use crate::url::ServerUrl;

/// How long a connection thread waits for a frame before it looks at its queue.
pub(crate) const POLL: Duration = Duration::from_millis(4);

/// A write that cannot finish in this long ends the connection (a peer that stopped reading).
pub(crate) const WRITE_TIMEOUT: Duration = Duration::from_secs(30);

/// Time allowed for connecting and for the handshake.
pub(crate) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(15);

/// Something for a connection thread to send.
#[derive(Debug)]
pub(crate) enum Out {
    /// A frame.
    Frame(Message),
    /// Close the connection, saying why.
    Close(String),
}

/// A stream whose underlying TCP socket's timeouts can be set.
pub(crate) trait Socket: Read + Write {
    /// The TCP socket underneath.
    fn tcp(&self) -> &TcpStream;
}

impl Socket for TcpStream {
    fn tcp(&self) -> &TcpStream {
        self
    }
}

/// A server-side stream that first replays the bytes already read while looking at the HTTP
/// request (the WebSocket handshake reads the request again).
pub(crate) struct Prefixed {
    head: Vec<u8>,
    at: usize,
    inner: TcpStream,
}

impl Prefixed {
    pub(crate) fn new(head: Vec<u8>, inner: TcpStream) -> Self {
        Self { head, at: 0, inner }
    }
}

impl Read for Prefixed {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.at < self.head.len() {
            let n = (self.head.len() - self.at).min(buf.len());
            buf[..n].copy_from_slice(&self.head[self.at..self.at + n]);
            self.at += n;
            return Ok(n);
        }
        self.inner.read(buf)
    }
}

impl Write for Prefixed {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl Socket for Prefixed {
    fn tcp(&self) -> &TcpStream {
        &self.inner
    }
}

/// A client's stream: plain, or TLS for `https://` servers.
pub(crate) enum ClientStream {
    /// `ws://`.
    Plain(TcpStream),
    /// `wss://`.
    #[cfg(feature = "tls")]
    Tls(Box<rustls::StreamOwned<rustls::ClientConnection, TcpStream>>),
}

impl Read for ClientStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Plain(s) => s.read(buf),
            #[cfg(feature = "tls")]
            Self::Tls(s) => s.read(buf),
        }
    }
}

impl Write for ClientStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Plain(s) => s.write(buf),
            #[cfg(feature = "tls")]
            Self::Tls(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Plain(s) => s.flush(),
            #[cfg(feature = "tls")]
            Self::Tls(s) => s.flush(),
        }
    }
}

impl Socket for ClientStream {
    fn tcp(&self) -> &TcpStream {
        match self {
            Self::Plain(s) => s,
            #[cfg(feature = "tls")]
            Self::Tls(s) => s.get_ref(),
        }
    }
}

/// Wraps a TCP connection in TLS for `host`, trusting the operating system's root certificates.
#[cfg(feature = "tls")]
fn tls(host: &str, tcp: TcpStream) -> Result<ClientStream, NetError> {
    use std::sync::Arc;
    let found = rustls_native_certs::load_native_certs();
    let mut roots = rustls::RootCertStore::empty();
    for cert in found.certs {
        // A certificate the store cannot use is skipped, as browsers do.
        let _ = roots.add(cert);
    }
    if roots.is_empty() {
        return Err(NetError::Tls(
            "no root certificates found in the operating system's store".into(),
        ));
    }
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| NetError::Tls(e.to_string()))?
        .with_root_certificates(roots)
        .with_no_client_auth();
    let name = rustls::pki_types::ServerName::try_from(host.to_owned())
        .map_err(|e| NetError::Tls(format!("{host}: {e}")))?;
    let connection = rustls::ClientConnection::new(Arc::new(config), name)
        .map_err(|e| NetError::Tls(e.to_string()))?;
    Ok(ClientStream::Tls(Box::new(rustls::StreamOwned::new(
        connection, tcp,
    ))))
}

#[cfg(not(feature = "tls"))]
fn tls(host: &str, _tcp: TcpStream) -> Result<ClientStream, NetError> {
    Err(NetError::Tls(format!(
        "{host}: this build cannot open https:// addresses (built without the `tls` feature)"
    )))
}

/// Connects to a server's WebSocket.
pub(crate) fn connect(
    url: &ServerUrl,
    config: WebSocketConfig,
) -> Result<WebSocket<ClientStream>, NetError> {
    let mut last = None;
    let mut tcp = None;
    for addr in (url.host.as_str(), url.connect_port()).to_socket_addrs()? {
        match TcpStream::connect_timeout(&addr, HANDSHAKE_TIMEOUT) {
            Ok(stream) => {
                tcp = Some(stream);
                break;
            }
            Err(error) => last = Some(error),
        }
    }
    let tcp = match (tcp, last) {
        (Some(tcp), _) => tcp,
        (None, Some(error)) => return Err(error.into()),
        (None, None) => return Err(NetError::Url(format!("{}: no address", url.host))),
    };
    tcp.set_nodelay(true)?;
    tcp.set_read_timeout(Some(HANDSHAKE_TIMEOUT))?;
    tcp.set_write_timeout(Some(WRITE_TIMEOUT))?;
    let stream = if url.secure {
        tls(&url.host, tcp)?
    } else {
        ClientStream::Plain(tcp)
    };
    let (ws, _) = tungstenite::client::client_with_config(url.ws_url(), stream, Some(config))
        .map_err(|error| match error {
            tungstenite::HandshakeError::Failure(e) => NetError::Ws(e.to_string()),
            tungstenite::HandshakeError::Interrupted(_) => {
                NetError::Ws("the handshake timed out".into())
            }
        })?;
    Ok(ws)
}

/// Runs a connection until either side closes it: queued frames are written, received ones
/// handed to `on_frame` (an error closes the connection with that reason). Returns why it
/// ended.
pub(crate) fn pump<S: Socket>(
    ws: &mut WebSocket<S>,
    outbound: &Receiver<Out>,
    mut on_frame: impl FnMut(Message) -> Result<(), String>,
) -> String {
    if let Err(error) = ws.get_ref().tcp().set_read_timeout(Some(POLL)) {
        return error.to_string();
    }
    let close = |ws: &mut WebSocket<S>, reason: &str| {
        let frame = CloseFrame {
            code: CloseCode::Normal,
            reason: reason.chars().take(100).collect::<String>().into(),
        };
        let _ = ws.close(Some(frame));
        let _ = ws.flush();
    };
    loop {
        let mut wrote = false;
        loop {
            match outbound.try_recv() {
                Ok(Out::Frame(message)) => {
                    if let Err(error) = ws.write(message) {
                        return error.to_string();
                    }
                    wrote = true;
                }
                Ok(Out::Close(reason)) => {
                    close(ws, &reason);
                    return reason;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    close(ws, "closed");
                    return "closed".into();
                }
            }
        }
        if wrote && let Err(error) = ws.flush() {
            return error.to_string();
        }
        match ws.read() {
            Ok(Message::Close(frame)) => {
                let _ = ws.flush();
                return frame.map_or_else(
                    || "closed by the other side".into(),
                    |f| format!("closed by the other side: {}", f.reason),
                );
            }
            Ok(message) => {
                if let Err(reason) = on_frame(message) {
                    close(ws, &reason);
                    return reason;
                }
            }
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) => {}
            Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => {
                return "closed".into();
            }
            Err(error) => return error.to_string(),
        }
    }
}

/// The request line and headers of an HTTP request.
pub(crate) struct Head {
    /// Every byte read so far (the request and anything after it).
    pub bytes: Vec<u8>,
    /// `GET`, …
    pub method: String,
    /// The path, without a query.
    pub path: String,
    /// Header names in lower case, with their values.
    pub headers: Vec<(String, String)>,
}

impl Head {
    /// A header's value.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

/// Longest request head accepted.
const HEAD_LIMIT: usize = 16 * 1024;

/// Reads an HTTP request's head.
pub(crate) fn read_head(stream: &mut TcpStream) -> Result<Head, String> {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 2048];
    loop {
        let n = stream.read(&mut chunk).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("closed before the request ended".into());
        }
        bytes.extend_from_slice(&chunk[..n]);
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut request = httparse::Request::new(&mut headers);
        match request.parse(&bytes) {
            Ok(httparse::Status::Complete(_)) => {
                let path = request.path.unwrap_or("/");
                let path = path.split('?').next().unwrap_or("/").to_owned();
                let method = request.method.unwrap_or("").to_owned();
                let headers = request
                    .headers
                    .iter()
                    .map(|h| {
                        (
                            h.name.to_ascii_lowercase(),
                            String::from_utf8_lossy(h.value).into_owned(),
                        )
                    })
                    .collect();
                return Ok(Head {
                    bytes,
                    method,
                    path,
                    headers,
                });
            }
            Ok(httparse::Status::Partial) if bytes.len() < HEAD_LIMIT => {}
            Ok(httparse::Status::Partial) => return Err("request head too long".into()),
            Err(error) => return Err(error.to_string()),
        }
    }
}

/// Writes a small HTTP response and ends it.
pub(crate) fn respond(stream: &mut TcpStream, status: &str, content_type: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}
