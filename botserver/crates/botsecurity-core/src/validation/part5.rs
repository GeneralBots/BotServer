use super::*;

pub fn validate_url_ssrf(url: &str) -> Result<(), ValidationError> {
    validate_url(url)?;

    let url_lower = url.to_lowercase();

    for blocked in SSRF_BLOCKED_HOSTS.iter() {
        if url_lower.contains(blocked) {
            return Err(ValidationError::Forbidden {
                field: "url".to_string(),
                reason: format!("URL contains blocked host or pattern: {}", blocked),
            });
        }
    }

    if let Ok(parsed) = url::Url::parse(url) {
        let host_str: &str = match parsed.host_str() {
            Some(h) => h,
            None => {
                return Err(ValidationError::InvalidUrl(url.to_string()));
            }
        };

        if let Ok(addr) = host_str.parse::<IpAddr>() {
            let is_private = match addr {
                IpAddr::V4(ipv4) => ipv4.is_loopback() || ipv4.is_private() || ipv4.is_link_local(),
                IpAddr::V6(ipv6) => ipv6.is_loopback() || ipv6.is_unspecified(),
            };

            if is_private {
                return Err(ValidationError::Forbidden {
                    field: "url".to_string(),
                    reason: format!("URL resolves to private/internal address: {}", addr),
                });
            }
        }
    }

    Ok(())
}
