//! Tools > Export Public Key from Certificate: the public key of an X.509 certificate (PEM, DER, or a PFX / P12 file
//! read by Windows) as one OpenSSH `authorized_keys` line.

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let n = (chunk[0] as u32) << 16
            | (*chunk.get(1).unwrap_or(&0) as u32) << 8
            | *chunk.get(2).unwrap_or(&0) as u32;
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            B64[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

pub fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut acc = 0u32;
    let mut bits = 0;
    for c in text.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' | b'\r' | b'\n' | b' ' | b'\t' => continue,
            _ => return None,
        };
        acc = acc << 6 | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// One DER element: (tag, content, what follows it).
fn tlv(b: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let tag = *b.first()?;
    let l0 = *b.get(1)? as usize;
    let (len, off) = if l0 < 0x80 {
        (l0, 2)
    } else {
        let n = l0 & 0x7f;
        if n == 0 || n > 4 {
            return None;
        }
        let mut len = 0usize;
        for i in 0..n {
            len = len << 8 | *b.get(2 + i)? as usize;
        }
        (len, 2 + n)
    };
    let end = off.checked_add(len)?;
    Some((tag, b.get(off..end)?, b.get(end..)?))
}

fn ssh_string(out: &mut Vec<u8>, s: &[u8]) {
    out.extend_from_slice(&(s.len() as u32).to_be_bytes());
    out.extend_from_slice(s);
}

fn ssh_mpint(out: &mut Vec<u8>, int: &[u8]) {
    let mut i = 0;
    while i + 1 < int.len() && int[i] == 0 {
        i += 1;
    }
    let v = &int[i..];
    if v.first().is_some_and(|b| b & 0x80 != 0) {
        out.extend_from_slice(&((v.len() + 1) as u32).to_be_bytes());
        out.push(0);
    } else {
        out.extend_from_slice(&(v.len() as u32).to_be_bytes());
    }
    out.extend_from_slice(v);
}

const OID_RSA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const OID_EC: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01];
const OID_P256: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07];
const OID_P384: &[u8] = &[0x2b, 0x81, 0x04, 0x00, 0x22];
const OID_P521: &[u8] = &[0x2b, 0x81, 0x04, 0x00, 0x23];
const OID_ED25519: &[u8] = &[0x2b, 0x65, 0x70];

/// The SubjectPublicKeyInfo of a certificate in DER.
fn spki_of_certificate(der: &[u8]) -> Option<&[u8]> {
    let (_, cert, _) = tlv(der)?;
    let (_, tbs, _) = tlv(cert)?;
    let mut rest = tbs;
    let (tag, _, after) = tlv(rest)?;
    if tag == 0xa0 {
        rest = after; // version
    }
    // serial, signature algorithm, issuer, validity, subject
    for _ in 0..5 {
        rest = tlv(rest)?.2;
    }
    let (tag, _, _) = tlv(rest)?;
    if tag != 0x30 {
        return None;
    }
    // the whole SPKI element including its header
    let (_, content, after) = tlv(rest)?;
    let total = rest.len() - after.len();
    let _ = content;
    Some(&rest[..total])
}

/// The key of a SubjectPublicKeyInfo as an SSH public key blob and its algorithm name.
fn ssh_blob(spki: &[u8]) -> Result<(String, Vec<u8>), String> {
    let bad = || "The certificate's public key could not be read.".to_string();
    let (_, spki_c, _) = tlv(spki).ok_or_else(bad)?;
    let (_, alg, rest) = tlv(spki_c).ok_or_else(bad)?;
    let (tag, bits, _) = tlv(rest).ok_or_else(bad)?;
    if tag != 0x03 || bits.is_empty() {
        return Err(bad());
    }
    let key = &bits[1..];
    let (_, oid, params) = tlv(alg).ok_or_else(bad)?;
    let mut blob = Vec::new();
    if oid == OID_RSA {
        let (_, seq, _) = tlv(key).ok_or_else(bad)?;
        let (_, n, rest) = tlv(seq).ok_or_else(bad)?;
        let (_, e, _) = tlv(rest).ok_or_else(bad)?;
        ssh_string(&mut blob, b"ssh-rsa");
        ssh_mpint(&mut blob, e);
        ssh_mpint(&mut blob, n);
        Ok(("ssh-rsa".into(), blob))
    } else if oid == OID_EC {
        let (_, curve, _) = tlv(params).ok_or_else(bad)?;
        let name = if curve == OID_P256 {
            "nistp256"
        } else if curve == OID_P384 {
            "nistp384"
        } else if curve == OID_P521 {
            "nistp521"
        } else {
            return Err("This elliptic curve is not supported by OpenSSH.".into());
        };
        let alg = format!("ecdsa-sha2-{name}");
        ssh_string(&mut blob, alg.as_bytes());
        ssh_string(&mut blob, name.as_bytes());
        ssh_string(&mut blob, key);
        Ok((alg, blob))
    } else if oid == OID_ED25519 {
        ssh_string(&mut blob, b"ssh-ed25519");
        ssh_string(&mut blob, key);
        Ok(("ssh-ed25519".into(), blob))
    } else {
        Err("The certificate's key type is not supported (RSA, ECDSA and Ed25519 are).".into())
    }
}

fn certificate_der(path: &str) -> Result<Vec<u8>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Could not read the file: {e}"))?;
    let lower = path.to_lowercase();
    if lower.ends_with(".pfx") || lower.ends_with(".p12") {
        // Windows reads the container (without a password); the certificate comes back as base64
        let script = format!(
            "$c = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2('{}'); [Console]::Out.Write([Convert]::ToBase64String($c.RawData))",
            path.replace('\'', "''")
        );
        let out = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
            .output()
            .map_err(|e| format!("Could not start PowerShell: {e}"))?;
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        return base64_decode(&text).filter(|d| !d.is_empty()).ok_or_else(|| "The file could not be read as a certificate (a PFX with a password is not supported).".to_string());
    }
    if let Ok(text) = std::str::from_utf8(&bytes) {
        if let Some(start) = text.find("-----BEGIN CERTIFICATE-----") {
            let body = &text[start + "-----BEGIN CERTIFICATE-----".len()..];
            let end = body
                .find("-----END CERTIFICATE-----")
                .ok_or("The certificate file is incomplete.")?;
            return base64_decode(&body[..end])
                .ok_or_else(|| "The certificate file is not valid base64.".to_string());
        }
    }
    if bytes.first() == Some(&0x30) {
        return Ok(bytes);
    }
    Err("The file is not a certificate (PEM, DER, PFX or P12 expected).".into())
}

/// The `authorized_keys` line for the certificate in `path`.
pub fn public_key_from_certificate(path: &str) -> Result<String, String> {
    let der = certificate_der(path)?;
    let spki = spki_of_certificate(&der).ok_or("The file does not contain a valid certificate.")?;
    let (alg, blob) = ssh_blob(spki)?;
    let comment = std::path::Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(format!("{alg} {} {comment}", base64_encode(&blob)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_roundtrip() {
        for n in 0..20u8 {
            let d: Vec<u8> = (0..n).map(|i| i.wrapping_mul(37)).collect();
            assert_eq!(base64_decode(&base64_encode(&d)).unwrap(), d);
        }
    }

    fn line_for(file: &str) -> String {
        let p = format!("{}/src/testdata/{file}", env!("CARGO_MANIFEST_DIR"));
        public_key_from_certificate(&p).unwrap()
    }

    fn expected(file: &str) -> String {
        std::fs::read_to_string(format!(
            "{}/src/testdata/{file}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
        .trim()
        .to_string()
    }

    #[test]
    fn rsa_certificate_gives_the_same_key_as_ssh_keygen() {
        let got = line_for("rsa.crt");
        assert_eq!(got.rsplit_once(' ').unwrap().0, expected("rsa.pubkey"));
    }

    #[test]
    fn ecdsa_certificate_gives_the_same_key_as_ssh_keygen() {
        let got = line_for("ec.crt");
        assert_eq!(got.rsplit_once(' ').unwrap().0, expected("ec.pubkey"));
    }

    #[test]
    fn ed25519_certificate_is_an_ssh_ed25519_key() {
        let got = line_for("ed.crt");
        assert!(
            got.starts_with("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI"),
            "{got}"
        );
    }

    #[test]
    fn a_text_file_is_refused() {
        let p = std::env::temp_dir().join("yetty_not_a_cert.txt");
        std::fs::write(&p, "hello").unwrap();
        assert!(public_key_from_certificate(&p.to_string_lossy()).is_err());
    }
}
