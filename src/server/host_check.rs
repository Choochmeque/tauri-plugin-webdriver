//! Refuses a request not addressed to this server by its address.
//!
//! The server is unauthenticated, so the interfaces it binds are what keeps other parties out.
//! DNS rebinding gets around that: a web page in a browser points its own name at an address of
//! this server, 127.0.0.1 or, on Android, which binds every interface, the device's network
//! address. The browser then sends the request with that name as `Host` and the page's origin as
//! `Origin`. A rebound request therefore always carries a name in `Host`, while a client that
//! reaches the server by its address carries the address.
//!
//! A request is served only when its `Host` is an IP literal (`127.0.0.1:<port>`,
//! `192.168.1.20:<port>`, `[::1]:<port>`) or `localhost:<port>`, `<port>` the bound one, and its
//! `Origin` is absent or `http://` and that same `Host`. Any other request, one without `Host` or
//! with a header that is not visible ASCII included, gets 403 Forbidden. A `WebDriver` client
//! sends `Host` as the URL it was given names the server, and no `Origin`.

use std::net::SocketAddr;

use axum::extract::{Request, State};
use axum::http::header::ToStrError;
use axum::http::header::{HOST, ORIGIN};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

const LOCALHOST: &str = "localhost";

/// Serves `request` only when it is addressed to this server on `port`.
pub async fn refuse_foreign_requests(
    State(port): State<u16>,
    request: Request,
    next: Next,
) -> Response {
    let headers = request.headers();
    let (Ok(host), Ok(origin)) = (header_text(headers, HOST), header_text(headers, ORIGIN)) else {
        return StatusCode::FORBIDDEN.into_response();
    };
    if is_addressed_to_this_server(host, origin, port) {
        next.run(request).await
    } else {
        StatusCode::FORBIDDEN.into_response()
    }
}

fn header_text(headers: &HeaderMap, name: HeaderName) -> Result<Option<&str>, ToStrError> {
    headers.get(name).map(HeaderValue::to_str).transpose()
}

fn is_addressed_to_this_server(host: Option<&str>, origin: Option<&str>, port: u16) -> bool {
    let Some(host) = host else {
        return false;
    };
    let is_same_origin = origin.is_none_or(|origin| origin.strip_prefix("http://") == Some(host));
    names_this_server(host, port) && is_same_origin
}

fn names_this_server(authority: &str, port: u16) -> bool {
    if let Ok(address) = authority.parse::<SocketAddr>() {
        return address.port() == port;
    }
    authority
        .rsplit_once(':')
        .is_some_and(|(name, named_port)| {
            name.eq_ignore_ascii_case(LOCALHOST) && named_port == port.to_string()
        })
}

#[cfg(test)]
mod tests {
    use super::is_addressed_to_this_server;

    const PORT: u16 = 4445;

    #[test]
    fn a_foreign_name_is_refused() {
        assert!(!is_addressed_to_this_server(
            Some("evil.example:4445"),
            None,
            PORT
        ));
    }

    #[test]
    fn the_loopback_address_with_the_bound_port_is_allowed() {
        assert!(is_addressed_to_this_server(
            Some("127.0.0.1:4445"),
            None,
            PORT
        ));
    }

    #[test]
    fn localhost_with_the_bound_port_is_allowed() {
        assert!(is_addressed_to_this_server(
            Some("localhost:4445"),
            None,
            PORT
        ));
    }

    #[test]
    fn a_network_address_with_the_bound_port_is_allowed() {
        assert!(is_addressed_to_this_server(
            Some("192.168.1.20:4445"),
            None,
            PORT
        ));
    }

    #[test]
    fn an_ipv6_address_with_the_bound_port_is_allowed() {
        assert!(is_addressed_to_this_server(Some("[::1]:4445"), None, PORT));
    }

    #[test]
    fn an_address_with_another_port_is_refused() {
        assert!(!is_addressed_to_this_server(
            Some("127.0.0.1:4446"),
            None,
            PORT
        ));
    }

    #[test]
    fn localhost_with_another_port_is_refused() {
        assert!(!is_addressed_to_this_server(
            Some("localhost:4446"),
            None,
            PORT
        ));
    }

    #[test]
    fn an_address_without_a_port_is_refused() {
        assert!(!is_addressed_to_this_server(Some("127.0.0.1"), None, PORT));
    }

    #[test]
    fn a_request_without_a_host_is_refused() {
        assert!(!is_addressed_to_this_server(None, None, PORT));
    }

    #[test]
    fn a_foreign_origin_is_refused_even_with_an_address_host() {
        assert!(!is_addressed_to_this_server(
            Some("127.0.0.1:4445"),
            Some("http://evil.example"),
            PORT
        ));
    }

    #[test]
    fn an_origin_at_another_address_is_refused() {
        assert!(!is_addressed_to_this_server(
            Some("192.168.1.20:4445"),
            Some("http://203.0.113.5:4445"),
            PORT
        ));
    }

    #[test]
    fn an_https_origin_is_refused() {
        assert!(!is_addressed_to_this_server(
            Some("127.0.0.1:4445"),
            Some("https://127.0.0.1:4445"),
            PORT
        ));
    }

    #[test]
    fn the_origin_of_the_host_itself_is_allowed() {
        assert!(is_addressed_to_this_server(
            Some("192.168.1.20:4445"),
            Some("http://192.168.1.20:4445"),
            PORT
        ));
    }

    #[test]
    fn the_localhost_origin_is_allowed() {
        assert!(is_addressed_to_this_server(
            Some("localhost:4445"),
            Some("http://localhost:4445"),
            PORT
        ));
    }
}
