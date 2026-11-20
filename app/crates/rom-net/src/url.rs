//! A server's address as people type it, and the WebSocket address it gives:
//! `http://host[:port][/path]` → `ws://…/ws`, `https://host[/path]` → `wss://…/ws`.

use crate::NetError;

/// A server's address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerUrl {
    /// `https` (or `wss`): TLS.
    pub secure: bool,
    /// Host name or address (an IPv6 address without brackets).
    pub host: String,
    /// The port, when given.
    pub port: Option<u16>,
    /// The path, beginning and ending with `/`.
    pub path: String,
}

impl ServerUrl {
    /// Reads `http://`, `https://`, `ws://` or `wss://` addresses, or a bare `host[:port]`
    /// (plain).
    pub fn parse(text: &str) -> Result<Self, NetError> {
        let bad = |why: &str| NetError::Url(format!("{text:?}: {why}"));
        let text = text.trim();
        let (secure, rest) = match text.split_once("://") {
            Some((scheme, rest)) => match scheme.to_ascii_lowercase().as_str() {
                "http" | "ws" => (false, rest),
                "https" | "wss" => (true, rest),
                _ => return Err(bad("the scheme must be http or https")),
            },
            None => (false, text),
        };
        let (authority, path) = match rest.find('/') {
            Some(at) => (&rest[..at], &rest[at..]),
            None => (rest, "/"),
        };
        let path = path.split(['?', '#']).next().unwrap_or("/");
        let (host, port) = if let Some(inner) = authority.strip_prefix('[') {
            let (host, after) = inner.split_once(']').ok_or_else(|| bad("unclosed ["))?;
            (host, after.strip_prefix(':'))
        } else {
            match authority.rsplit_once(':') {
                Some((host, port)) => (host, Some(port)),
                None => (authority, None),
            }
        };
        if host.is_empty() {
            return Err(bad("no host"));
        }
        let port = match port {
            Some(port) => Some(port.parse::<u16>().map_err(|_| bad("bad port"))?),
            None => None,
        };
        let mut path = path.to_owned();
        if !path.ends_with('/') {
            path.push('/');
        }
        Ok(Self {
            secure,
            host: host.to_owned(),
            port,
            path,
        })
    }

    /// `host[:port]` as it goes in a URL.
    fn authority(&self) -> String {
        let host = if self.host.contains(':') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        match self.port {
            Some(port) => format!("{host}:{port}"),
            None => host,
        }
    }

    /// The WebSocket address: the path followed by `ws`.
    pub fn ws_url(&self) -> String {
        let scheme = if self.secure { "wss" } else { "ws" };
        format!("{scheme}://{}{}ws", self.authority(), self.path)
    }

    /// The status page's address.
    pub fn status_url(&self) -> String {
        let scheme = if self.secure { "https" } else { "http" };
        format!("{scheme}://{}{}status", self.authority(), self.path)
    }

    /// The port to connect to.
    pub fn connect_port(&self) -> u16 {
        self.port.unwrap_or(if self.secure { 443 } else { 80 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws(text: &str) -> String {
        ServerUrl::parse(text).unwrap().ws_url()
    }

    #[test]
    fn the_url_rules() {
        assert_eq!(ws("http://h:1/p"), "ws://h:1/p/ws");
        assert_eq!(ws("https://h/p/"), "wss://h/p/ws");
        assert_eq!(ws("http://127.0.0.1:7400/"), "ws://127.0.0.1:7400/ws");
        assert_eq!(ws("http://127.0.0.1:7400"), "ws://127.0.0.1:7400/ws");
        assert_eq!(
            ws("https://example.org/ascension/"),
            "wss://example.org/ascension/ws"
        );
        assert_eq!(ws("HTTPS://h"), "wss://h/ws");
        assert_eq!(ws("localhost:7400"), "ws://localhost:7400/ws");
        assert_eq!(ws("http://[::1]:7400/a/b?x=1"), "ws://[::1]:7400/a/b/ws");
        let url = ServerUrl::parse("https://h/p").unwrap();
        assert_eq!(url.connect_port(), 443);
        assert_eq!(url.status_url(), "https://h/p/status");
        assert_eq!(ServerUrl::parse("http://h").unwrap().connect_port(), 80);
        assert!(ServerUrl::parse("ftp://h").is_err());
        assert!(ServerUrl::parse("http://:80/").is_err());
        assert!(ServerUrl::parse("http://h:port/").is_err());
    }
}
