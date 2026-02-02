use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use libp2p::Multiaddr;
use std::str::FromStr;

pub fn generate_void_code(multiaddr: &Multiaddr) -> String {
    let s = multiaddr.to_string();
    let encoded = BASE64.encode(s);
    format!("void://{}", encoded)
}

pub fn parse_void_code(code: &str) -> Result<Multiaddr, String> {
    let code = code.trim();
    
    if !code.starts_with("void://") {
        return Err("Invalid protocol prefix".to_string());
    }

    let encoded = code.trim_start_matches("void://");
    let decoded_vec = BASE64
        .decode(encoded)
        .map_err(|e| format!("Base64 decode error: {}", e))?;
    let decoded_str =
        String::from_utf8(decoded_vec).map_err(|e| format!("UTF-8 decode error: {}", e))?;

    Multiaddr::from_str(&decoded_str).map_err(|e| format!("Invalid Multiaddr: {}", e))
}
