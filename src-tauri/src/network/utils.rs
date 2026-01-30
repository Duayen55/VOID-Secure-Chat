use libp2p::{PeerId, Multiaddr};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::str::FromStr;

pub fn generate_void_code(peer_id: PeerId, public_ip: &str, port: u16) -> String {
    let s = format!("void://{}@{}:{}", peer_id, public_ip, port);
    BASE64.encode(s)
}

pub fn parse_void_code(code: &str) -> Result<Multiaddr, String> {
    let code = code.trim();
    let decoded_vec = BASE64.decode(code).map_err(|e| format!("Base64 decode error: {}", e))?;
    let decoded = String::from_utf8(decoded_vec).map_err(|e| format!("UTF-8 decode error: {}", e))?;

    if !decoded.starts_with("void://") {
        return Err("Invalid protocol prefix".to_string());
    }

    let content = decoded.trim_start_matches("void://");
    let parts: Vec<&str> = content.split('@').collect();
    if parts.len() != 2 {
        return Err("Invalid format. Expected void://<peer_id>@<ip>:<port>".to_string());
    }

    let peer_id_str = parts[0];
    let addr_str = parts[1];
    
    // Handle IPv6 which contains colons
    let (ip, port) = if let Some(idx) = addr_str.rfind(':') {
        (&addr_str[..idx], &addr_str[idx+1..])
    } else {
        return Err("Invalid address format. Missing port.".to_string());
    };

    // Construct Multiaddr
    // Prioritize QUIC as per user request
    let ma_str = format!("/ip4/{}/udp/{}/quic-v1/p2p/{}", ip, port, peer_id_str);
    
    Multiaddr::from_str(&ma_str).map_err(|e| format!("Invalid Multiaddr construction: {}", e))
}
