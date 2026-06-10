#![allow(clippy::collapsible_if, clippy::collapsible_else_if, clippy::map_unwrap_or, clippy::needless_bool)]
//! provenance.rs -- SLSA v1.0 build provenance as a signed in-toto Statement.
//!
//! Every successful build emits `<base>.provenance.json`: an in-toto Statement
//! (`_type` https://in-toto.io/Statement/v1) carrying the artifact digest, a
//! SLSA v1.0 predicate (buildDefinition + runDetails), and an Ed25519 signature
//! over the canonical body -- reusing the same key as the .zcert. The signature +
//! pubkey are the last two fields, so `cert_sign::verify_cert_file` verifies it too.

use crate::cert_sign;
use sha2::Digest;

fn sha256_file(path: &str) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let mut h = sha2::Sha256::new();
    h.update(&bytes);
    Some(h.finalize().iter().map(|b| format!("{:02x}", b)).collect())
}

fn json_escape(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

/// Unix seconds -> RFC3339 UTC timestamp, no external date dependency
/// (civil_from_days, Howard Hinnant's algorithm).
fn rfc3339(secs: u64) -> String {
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, m, d, h, mi, s)
}

/// Emit and sign `<base>.provenance.json` for a just-built artifact.
pub fn write_provenance(source_path: &str, base_name: &str) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let ts = rfc3339(now);
    let invocation_id = format!("{:x}-{}", now, std::process::id());

    // Subject = the produced artifact: prefer the native binary, else the emitted C.
    let (subject_name, subject_digest) = if let Some(h) = sha256_file(base_name) {
        (base_name.to_string(), h)
    } else if let Some(h) = sha256_file(&format!("{}.c", base_name)) {
        (format!("{}.c", base_name), h)
    } else {
        (base_name.to_string(), String::new())
    };
    let src_digest = sha256_file(source_path).unwrap_or_default();
    let platform = format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH);

    // Canonical body: top-level fields one per line; predicate is a single line.
    // Ends just before the appended "signature" field (matches cert_sign markers).
    let body = format!(
        "{{\n  \"_type\":\"https://in-toto.io/Statement/v1\",\n  \"subject\":[{{\"name\":\"{}\",\"digest\":{{\"sha256\":\"{}\"}}}}],\n  \"predicateType\":\"https://slsa.dev/provenance/v1\",\n  \"predicate\":{{\"buildDefinition\":{{\"buildType\":\"https://zeus-lang.dev/buildtypes/compile/v1\",\"externalParameters\":{{\"source\":\"{}\",\"sourceSha256\":\"{}\"}},\"internalParameters\":{{\"compilerFlags\":\"-O2 -march=native\",\"builderPlatform\":\"{}\"}},\"resolvedDependencies\":[{{\"name\":\"{}\",\"digest\":{{\"sha256\":\"{}\"}}}}]}},\"runDetails\":{{\"builder\":{{\"id\":\"https://zeus-lang.dev/zeus_compiler@v0.1.0\"}},\"metadata\":{{\"invocationId\":\"{}\",\"startedOn\":\"{}\",\"finishedOn\":\"{}\"}}}}}},",
        json_escape(&subject_name), subject_digest,
        json_escape(source_path), src_digest, platform,
        json_escape(source_path), src_digest,
        invocation_id, ts, ts);

    let (sig_hex, pub_hex) = cert_sign::sign_body(body.as_bytes());
    let doc = format!("{}\n  \"signature\":\"{}\",\n  \"pubkey\":\"{}\"\n}}\n", body, sig_hex, pub_hex);
    let _ = std::fs::write(format!("{}.provenance.json", base_name), doc);
}
