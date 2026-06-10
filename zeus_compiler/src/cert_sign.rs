#![allow(clippy::collapsible_if, clippy::collapsible_else_if, clippy::map_unwrap_or, clippy::needless_bool)]
//! cert_sign.rs -- Ed25519 signing/verification for Zeus trust certificates.
//!
//! The .zcert already carries a SHA-256 content hash of the source. This module
//! adds an offline, dependency-light digital signature over the CANONICAL
//! certificate body (every field up to but not including the signature field),
//! so a verifier can detect tampering and confirm provenance.
//!
//! Key handling (offline, no network) -- PERSISTENT, STABLE IDENTITY:
//!   * The keypair lives in ONE stable directory so every build on a machine
//!     signs with the SAME identity (so a cert verifies across directories):
//!     $ZEUS_KEY_DIR  ->  $HOME/.zeus  ->  $USERPROFILE/.zeus  ->  "."(cwd)
//!     private key `zeus.key` (hex seed, 0600), public key `zeus.pub` (hex).
//!   * Override the signing seed directly with env ZEUS_SIGNING_KEY (32-byte hex).
//!   * Verification ALWAYS checks the signature against the cert's embedded
//!     pubkey. Provenance ("is this a signer I trust?") is a separate, opt-in
//!     check: set ZEUS_TRUSTED_PUB (hex or a path to a .pub) to HARD-FAIL on a
//!     signer mismatch. A differing local zeus.pub is only a note, never a fail.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use std::path::{Path, PathBuf};

fn key_dir() -> PathBuf {
    if let Ok(d) = std::env::var("ZEUS_KEY_DIR") { if !d.trim().is_empty() { return PathBuf::from(d); } }
    if let Ok(h) = std::env::var("HOME") { if !h.trim().is_empty() { return Path::new(&h).join(".zeus"); } }
    if let Ok(h) = std::env::var("USERPROFILE") { if !h.trim().is_empty() { return Path::new(&h).join(".zeus"); } }
    PathBuf::from(".")
}
fn key_path() -> PathBuf { key_dir().join("zeus.key") }
fn pub_path() -> PathBuf { key_dir().join("zeus.pub") }

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if !s.len().is_multiple_of(2) { return None; }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hi = (bytes[i] as char).to_digit(16)?;
        let lo = (bytes[i + 1] as char).to_digit(16)?;
        out.push((hi * 16 + lo) as u8);
        i += 2;
    }
    Some(out)
}

#[cfg(unix)]
fn write_private_key(path: &Path, hex: &str) {
    use std::os::unix::fs::OpenOptionsExt;
    use std::io::Write;
    match std::fs::OpenOptions::new()
        .write(true).create(true).truncate(true).mode(0o600).open(path)
    {
        Ok(mut f) => { let _ = f.write_all(hex.as_bytes()); }
        Err(e) => eprintln!("[ZEUS CERT] warning: could not write {} (0600): {}", path.display(), e),
    }
}

#[cfg(not(unix))]
fn write_private_key(path: &Path, hex: &str) {
    if let Err(e) = std::fs::write(path, hex) {
        eprintln!("[ZEUS CERT] warning: could not write {}: {}", path.display(), e);
    }
}

/// Load the persistent signing key (env > stable key file), else generate one
/// and persist it to the stable key directory so all future builds reuse it.
fn load_or_create_signing_key() -> SigningKey {
    // 1) Environment variable (hex of the 32-byte seed) -- highest priority.
    if let Ok(hex) = std::env::var("ZEUS_SIGNING_KEY") {
        if let Some(bytes) = hex_decode(&hex) {
            if bytes.len() == 32 {
                let seed: [u8; 32] = bytes.try_into().unwrap();
                return SigningKey::from_bytes(&seed);
            }
        }
        eprintln!("[ZEUS CERT] warning: ZEUS_SIGNING_KEY is not 32-byte hex; ignoring.");
    }
    // 2) Persistent key file in the stable key directory.
    let kp = key_path();
    if let Ok(hex) = std::fs::read_to_string(&kp) {
        if let Some(bytes) = hex_decode(&hex) {
            if bytes.len() == 32 {
                let seed: [u8; 32] = bytes.try_into().unwrap();
                return SigningKey::from_bytes(&seed);
            }
        }
        eprintln!("[ZEUS CERT] warning: {} is not 32-byte hex; regenerating.", kp.display());
    }
    // 3) Generate once, persist to the stable directory (created if needed).
    let _ = std::fs::create_dir_all(key_dir());
    let mut rng = rand_core::OsRng;
    let sk = SigningKey::generate(&mut rng);
    write_private_key(&kp, &hex_encode(&sk.to_bytes()));
    let _ = std::fs::write(pub_path(), hex_encode(sk.verifying_key().as_bytes()));
    eprintln!("[ZEUS CERT] generated a new persistent signing identity at {}", kp.display());
    sk
}

/// Sign the canonical body bytes and return (signature_hex, pubkey_hex).
pub fn sign_body(body: &[u8]) -> (String, String) {
    let sk = load_or_create_signing_key();
    let sig: Signature = sk.sign(body);
    (hex_encode(&sig.to_bytes()), hex_encode(sk.verifying_key().as_bytes()))
}

/// Verify a `.zcert` file: recompute the canonical body, then verify the embedded
/// Ed25519 signature against the embedded pubkey. Provenance (trusting the signer)
/// is opt-in via ZEUS_TRUSTED_PUB. A differing local zeus.pub is only a note.
pub fn verify_cert_file(path: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {}", path, e))?;

    let sig_hex = extract_field(&text, "signature")
        .ok_or_else(|| "certificate has no \"signature\" field".to_string())?;
    let pub_hex = extract_field(&text, "pubkey")
        .ok_or_else(|| "certificate has no \"pubkey\" field".to_string())?;

    let body = canonical_body(&text)
        .ok_or_else(|| "could not isolate canonical certificate body".to_string())?;

    let sig_bytes = hex_decode(&sig_hex).ok_or_else(|| "signature is not valid hex".to_string())?;
    let sig_arr: [u8; 64] = sig_bytes.as_slice().try_into()
        .map_err(|_| "signature is not 64 bytes".to_string())?;
    let signature = Signature::from_bytes(&sig_arr);

    let pub_bytes = hex_decode(&pub_hex).ok_or_else(|| "pubkey is not valid hex".to_string())?;
    let pub_arr: [u8; 32] = pub_bytes.as_slice().try_into()
        .map_err(|_| "pubkey is not 32 bytes".to_string())?;
    let vk = VerifyingKey::from_bytes(&pub_arr).map_err(|e| format!("invalid pubkey: {}", e))?;

    // Provenance policy:
    //  * ZEUS_TRUSTED_PUB set (hex or a path to a .pub) -> HARD-FAIL on mismatch.
    //  * otherwise, a differing stable zeus.pub is just a note (signature still
    //    proves integrity; trusting the signer is a separate decision).
    if let Ok(trusted) = std::env::var("ZEUS_TRUSTED_PUB") {
        let trusted_hex = if Path::new(&trusted).exists() {
            std::fs::read_to_string(&trusted).unwrap_or_default()
        } else { trusted.clone() };
        if !trusted_hex.trim().is_empty() && trusted_hex.trim() != pub_hex.trim() {
            return Err("embedded pubkey does not match ZEUS_TRUSTED_PUB (untrusted signer)".to_string());
        }
    } else if let Ok(local_pub) = std::fs::read_to_string(pub_path()) {
        if local_pub.trim() != pub_hex.trim() {
            eprintln!("[ZEUS CERT] note: this cert was signed by a key different from your local {} \
                       (the signature is valid; set ZEUS_TRUSTED_PUB to enforce a specific signer).",
                      pub_path().display());
        }
    }

    vk.verify(body.as_bytes(), &signature)
        .map_err(|e| format!("signature verification failed: {}", e))
}

/// Extract a top-level string field value (e.g. "signature":"<hex>").
fn extract_field(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":\"", key);
    let start = text.find(&needle)? + needle.len();
    let rest = &text[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Reconstruct the exact bytes that were signed: the certificate text up to (but
/// not including) the `"signature"` field line.
fn canonical_body(text: &str) -> Option<String> {
    let marker = "\n  \"signature\":";
    let idx = text.find(marker)?;
    Some(text[..idx].to_string())
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hex_roundtrip() {
        let b = vec![0u8, 1, 15, 16, 200, 254, 255];
        assert_eq!(hex_decode(&hex_encode(&b)).unwrap(), b);
    }
    #[test]
    fn sign_then_verify_and_detect_tamper() {
        let body = b"zeus canonical body";
        let (sig_hex, pub_hex) = sign_body(body);
        let sig_arr: [u8; 64] = hex_decode(&sig_hex).unwrap().as_slice().try_into().unwrap();
        let pub_arr: [u8; 32] = hex_decode(&pub_hex).unwrap().as_slice().try_into().unwrap();
        let sig = Signature::from_bytes(&sig_arr);
        let vk = VerifyingKey::from_bytes(&pub_arr).unwrap();
        assert!(vk.verify(body, &sig).is_ok());
        assert!(vk.verify(b"tampered body", &sig).is_err());
    }
    #[test]
    fn canonical_body_excludes_signature() {
        let cert = "{\n  \"a\":\"1\",\n  \"signature\":\"deadbeef\",\n  \"pubkey\":\"cafe\"\n}\n";
        assert_eq!(extract_field(cert, "signature").unwrap(), "deadbeef");
        let body = canonical_body(cert).unwrap();
        assert!(body.contains("\"a\":\"1\"") && !body.contains("signature"));
    }
}
