use log::debug;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::LazyLock;
use std::time::Duration;

pub const USER_AGENT: &str =
    "RetroPlayer/0.1.0 (https://github.com/The-Nice-One/PlaygroundProjects)";

const MAX_ATTEMPTS: usize = 3;
const TRANSPORT_BACKOFF_MS: [u64; 2] = [300, 800];

struct Ipv4FirstResolver;

pub(crate) fn sort_ipv4_first(addrs: &mut [SocketAddr]) {
    addrs.sort_by_key(|a| if a.is_ipv4() { 0u8 } else { 1u8 });
}

impl ureq::Resolver for Ipv4FirstResolver {
    fn resolve(&self, netloc: &str) -> std::io::Result<Vec<SocketAddr>> {
        let mut addrs: Vec<SocketAddr> = netloc.to_socket_addrs()?.collect();
        sort_ipv4_first(&mut addrs);
        Ok(addrs)
    }
}

static HTTP_AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
    ureq::AgentBuilder::new()
        .resolver(Ipv4FirstResolver)
        .timeout_connect(Duration::from_secs(5))
        .timeout_read(Duration::from_secs(10))
        .timeout_write(Duration::from_secs(5))
        .build()
});

/// Returns the shared, thread-safe `ureq::Agent` configured with HTTP Keep-Alive
/// connection pooling and an IPv4-first DNS resolver (preventing IPv6 TLS drops).
pub fn http_agent() -> &'static ureq::Agent {
    &HTTP_AGENT
}

pub(crate) fn parse_retry_delay_secs(
    retry_after: Option<&str>,
    rate_limit_reset_in: Option<&str>,
) -> u64 {
    let parsed = retry_after
        .or(rate_limit_reset_in)
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(1);
    parsed.clamp(1, 5)
}

/// Executes a request built against the shared `http_agent()`, retrying on
/// transient TLS/connection errors (`UnexpectedEof`, reset) and HTTP `429`/`503`
/// rate-limit responses.
pub fn call_with_retry<F>(build_req: F) -> anyhow::Result<ureq::Response>
where
    F: Fn(&ureq::Agent) -> ureq::Request,
{
    let agent = http_agent();

    for attempt in 0..MAX_ATTEMPTS {
        let req = build_req(agent);
        match req.call() {
            Ok(resp) => return Ok(resp),
            Err(ureq::Error::Status(status, resp)) => {
                if (status == 429 || status == 503) && attempt + 1 < MAX_ATTEMPTS {
                    let delay_secs = parse_retry_delay_secs(
                        resp.header("Retry-After"),
                        resp.header("X-RateLimit-Reset-In"),
                    );
                    debug!(
                        "HTTP {} from {}; retrying in {}s (attempt {}/{})",
                        status,
                        resp.get_url(),
                        delay_secs,
                        attempt + 1,
                        MAX_ATTEMPTS
                    );
                    std::thread::sleep(Duration::from_secs(delay_secs));
                    continue;
                }
                let url = resp.get_url().to_string();
                let body = resp.into_string().unwrap_or_default();
                anyhow::bail!("HTTP {} from {}: {}", status, url, body);
            }
            Err(ureq::Error::Transport(err)) => {
                if attempt + 1 < MAX_ATTEMPTS {
                    let backoff_ms = TRANSPORT_BACKOFF_MS[attempt.min(TRANSPORT_BACKOFF_MS.len() - 1)];
                    debug!(
                        "Transport error ({}); retrying in {}ms (attempt {}/{})",
                        err,
                        backoff_ms,
                        attempt + 1,
                        MAX_ATTEMPTS
                    );
                    std::thread::sleep(Duration::from_millis(backoff_ms));
                    continue;
                }
                return Err(anyhow::Error::from(err));
            }
        }
    }

    unreachable!("retry loop always returns on final attempt")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_sort_ipv4_first() {
        let v6 = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 443);
        let v4_a = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(142, 132, 241, 153)), 443);
        let v4_b = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 443);

        let mut addrs = vec![v6, v4_a, v4_b];
        sort_ipv4_first(&mut addrs);

        assert!(addrs[0].is_ipv4());
        assert!(addrs[1].is_ipv4());
        assert!(addrs[2].is_ipv6());
        assert_eq!(addrs[0], v4_a);
        assert_eq!(addrs[1], v4_b);
        assert_eq!(addrs[2], v6);
    }

    #[test]
    fn test_parse_retry_delay_clamping() {
        assert_eq!(parse_retry_delay_secs(None, None), 1);
        assert_eq!(parse_retry_delay_secs(Some("0"), None), 1);
        assert_eq!(parse_retry_delay_secs(Some("3"), None), 3);
        assert_eq!(parse_retry_delay_secs(Some("30"), None), 5);
        assert_eq!(parse_retry_delay_secs(None, Some("4")), 4);
        assert_eq!(parse_retry_delay_secs(Some("invalid"), Some("2")), 1);
    }
}
