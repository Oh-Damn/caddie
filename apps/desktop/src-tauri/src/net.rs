use std::net::IpAddr;
use std::process::{Command, Stdio};

pub fn mac_for_ip(ip: IpAddr) -> Option<String> {
    if ip.is_loopback() {
        return None;
    }
    let out = Command::new("arp")
        .args(["-n", &ip.to_string()])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_mac(&String::from_utf8_lossy(&out.stdout))
}

fn parse_mac(raw: &str) -> Option<String> {
    let at = raw.split(" at ").nth(1)?;
    let token = at.split_whitespace().next()?;
    normalize_mac(token)
}

fn normalize_mac(token: &str) -> Option<String> {
    let parts: Vec<&str> = token.split(':').collect();
    if parts.len() != 6 {
        return None;
    }
    let mut out = Vec::with_capacity(6);
    for part in parts {
        let byte = u8::from_str_radix(part, 16).ok()?;
        out.push(format!("{byte:02x}"));
    }
    let mac = out.join(":");
    if mac == "ff:ff:ff:ff:ff:ff" {
        return None;
    }
    Some(mac)
}

#[cfg(test)]
mod tests {
    use super::{normalize_mac, parse_mac};

    #[test]
    fn parses_macos_arp_line() {
        let line = "? (192.168.1.4) at 9e:d6:e3:cc:d3:c5 on en0 ifscope [ethernet]\n";
        assert_eq!(parse_mac(line).as_deref(), Some("9e:d6:e3:cc:d3:c5"));
    }

    #[test]
    fn pads_short_octets() {
        let line = "? (192.168.1.10) at 6:d:85:4c:5:53 on en0 ifscope [ethernet]\n";
        assert_eq!(parse_mac(line).as_deref(), Some("06:0d:85:4c:05:53"));
    }

    #[test]
    fn rejects_missing_and_broadcast() {
        assert!(parse_mac("192.168.1.222 (192.168.1.222) -- no entry\n").is_none());
        assert!(normalize_mac("ff:ff:ff:ff:ff:ff").is_none());
        assert!(normalize_mac("nope").is_none());
    }
}
