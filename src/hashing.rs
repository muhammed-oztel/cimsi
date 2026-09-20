use base64::Engine;
use sha2::Digest;

/// Hash `input` with the named algorithm and render it with the named encoding.
///
/// `algorithm` is one of "MD5", "SHA-1", "SHA-256"; `encoding` is "Hex" or "Base64".
/// Unknown values fall back to MD5 / Hex respectively, since both come from
/// fixed dropdown lists.
pub fn compute_hash(input: &str, algorithm: &str, encoding: &str) -> String {
    let bytes: Vec<u8> = match algorithm {
        "SHA-1" => sha1_smol::Sha1::from(input.as_bytes())
            .digest()
            .bytes()
            .to_vec(),
        "SHA-256" => sha2::Sha256::digest(input.as_bytes()).to_vec(),
        _ => md5::compute(input.as_bytes()).0.to_vec(),
    };

    match encoding {
        "Base64" => base64::engine::general_purpose::STANDARD.encode(&bytes),
        _ => hex_encode(&bytes),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;

    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(out, "{byte:02x}").unwrap();
    }
    out
}
