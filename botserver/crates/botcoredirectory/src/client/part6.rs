

/// True when the host provably stays inside a trusted perimeter: loopback,
/// RFC1918/ULA private addresses or link-local. DNS hostnames cannot be
/// verified as private without resolution (SSRF risk) and always need https.
pub(super) fn is_private_host(host: &str) -> bool {
    let h = host.trim_start_matches('[').trim_end_matches(']');
    if h == "localhost" || h.starts_with("127.") {
        return true;
    }
    if let Ok(addr) = h.parse::<std::net::IpAddr>() {
        return match addr {
            std::net::IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_link_local(),
            std::net::IpAddr::V6(v6) => {
                v6.is_loopback() || (v6.segments()[0] & 0xfe00) == 0xfc00 || v6.is_unicast_link_local()
            }
        };
    }
    false
}
