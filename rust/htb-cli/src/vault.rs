
use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use zeroize::{Zeroize, Zeroizing};

pub const MAGIC: &[u8; 6] = b"HTBHFC";
pub const VERSION: u8 = 1;
pub const KDF_ARGON2ID: u8 = 1;
pub const KDF_PBKDF2: u8 = 2;

const PREFIX_LEN: usize = 20;
const TAG_LEN: usize = 32;
const SALT_ARGON2: usize = 16;
const SALT_PBKDF2: usize = 8;
const OKM_LEN: usize = 80;

const ARGON2_MEMORY_KIB: u32 = 131_072;
const ARGON2_PASSES: u32 = 3;
const ARGON2_LANES: u32 = 4;

const PBKDF2_ROUNDS: u32 = 600_000;

pub const MIN_PASSPHRASE: usize = 12;

const PAD_TO: usize = 64;

pub type Secret = Zeroizing<String>;

#[derive(Debug)]
pub enum VaultError {
    Corrupt,
    WrongPassphrase,
    UnsupportedKdf(u8),
    UnsupportedVersion(u8),
    Crypto,
}

impl std::fmt::Display for VaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VaultError::Corrupt => write!(f, "el vault esta corrupto o no es un vault"),
            VaultError::WrongPassphrase => write!(f, "passphrase incorrecta"),
            VaultError::UnsupportedKdf(k) => write!(f, "derivacion de clave desconocida ({k})"),
            VaultError::UnsupportedVersion(v) => write!(f, "version de vault desconocida ({v})"),
            VaultError::Crypto => write!(f, "fallo al cifrar o descifrar"),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Kdf {
    pub id: u8,
    pub p1: u32,
    pub p2: u32,
    pub p3: u32,
}

impl Kdf {
    pub fn argon2id() -> Self {
        Kdf {
            id: KDF_ARGON2ID,
            p1: ARGON2_MEMORY_KIB,
            p2: ARGON2_PASSES,
            p3: ARGON2_LANES,
        }
    }

    pub fn pbkdf2() -> Self {
        Kdf {
            id: KDF_PBKDF2,
            p1: PBKDF2_ROUNDS,
            p2: 0,
            p3: 0,
        }
    }

    pub fn salt_len(&self) -> usize {
        match self.id {
            KDF_ARGON2ID => SALT_ARGON2,
            _ => SALT_PBKDF2,
        }
    }

    pub fn label(&self) -> String {
        match self.id {
            KDF_ARGON2ID => format!(
                "Argon2id m={}MiB t={} p={}",
                self.p1 / 1024,
                self.p2,
                self.p3
            ),
            KDF_PBKDF2 => format!("PBKDF2-HMAC-SHA512 {} rondas", self.p1),
            other => format!("kdf {other}"),
        }
    }

    fn validate(&self) -> Result<(), VaultError> {
        let sane = match self.id {
            KDF_ARGON2ID => {
                self.p1 >= 8 * 1024 && self.p2 >= 1 && self.p3 >= 1 && self.p3 <= 64
            }
            KDF_PBKDF2 => self.p1 >= 100_000,
            _ => return Err(VaultError::UnsupportedKdf(self.id)),
        };
        if sane {
            Ok(())
        } else {
            Err(VaultError::Corrupt)
        }
    }

    fn derive(&self, passphrase: &str, salt: &[u8]) -> Result<Zeroizing<[u8; OKM_LEN]>, VaultError> {
        self.validate()?;
        let mut okm = Zeroizing::new([0u8; OKM_LEN]);
        match self.id {
            KDF_ARGON2ID => {
                let params = argon2::Params::new(self.p1, self.p2, self.p3, Some(OKM_LEN))
                    .map_err(|_| VaultError::Corrupt)?;
                let argon = argon2::Argon2::new(
                    argon2::Algorithm::Argon2id,
                    argon2::Version::V0x13,
                    params,
                );
                argon
                    .hash_password_into(passphrase.as_bytes(), salt, okm.as_mut())
                    .map_err(|_| VaultError::Crypto)?;
            }
            KDF_PBKDF2 => {
                pbkdf2::pbkdf2_hmac::<sha2::Sha512>(
                    passphrase.as_bytes(),
                    salt,
                    self.p1,
                    okm.as_mut(),
                );
            }
            other => return Err(VaultError::UnsupportedKdf(other)),
        }
        Ok(okm)
    }
}

fn random(buf: &mut [u8]) -> Result<(), VaultError> {
    getrandom::getrandom(buf).map_err(|_| VaultError::Crypto)
}

pub fn seal(token: &str, passphrase: &str, kdf: Kdf) -> Result<Vec<u8>, VaultError> {
    kdf.validate()?;
    let mut salt = vec![0u8; kdf.salt_len()];
    random(&mut salt)?;

    let okm = kdf.derive(passphrase, &salt)?;
    let mut plain = pad_plaintext(token)?;

    let ciphertext = {
        use aes::cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyIvInit};
        type Enc = cbc::Encryptor<aes::Aes256>;
        let enc = Enc::new_from_slices(&okm[..32], &okm[32..48]).map_err(|_| VaultError::Crypto)?;
        let claro = plain.len();
        let mut buf = vec![0u8; claro + 16];
        buf[..claro].copy_from_slice(&plain);
        let out = enc
            .encrypt_padded_mut::<Pkcs7>(&mut buf, claro)
            .map_err(|_| VaultError::Crypto)?;
        out.to_vec()
    };
    plain.zeroize();

    let header_len = PREFIX_LEN + salt.len() + 4;
    let mut blob = Vec::with_capacity(header_len + ciphertext.len() + TAG_LEN);
    blob.extend_from_slice(MAGIC);
    blob.push(VERSION);
    blob.push(kdf.id);
    blob.extend_from_slice(&kdf.p1.to_be_bytes());
    blob.extend_from_slice(&kdf.p2.to_be_bytes());
    blob.extend_from_slice(&kdf.p3.to_be_bytes());
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(&(ciphertext.len() as u32).to_be_bytes());
    blob.extend_from_slice(&ciphertext);

    let tag = mac(&okm[48..], &blob)?;
    blob.extend_from_slice(&tag);
    Ok(blob)
}

pub fn open(blob: &[u8], passphrase: &str) -> Result<Secret, VaultError> {
    let kdf = describe(blob).ok_or(VaultError::Corrupt)?;
    if blob[6] != VERSION {
        return Err(VaultError::UnsupportedVersion(blob[6]));
    }
    let salt_len = kdf.salt_len();
    let header_len = PREFIX_LEN + salt_len + 4;
    if blob.len() < header_len + TAG_LEN {
        return Err(VaultError::Corrupt);
    }
    let salt = &blob[PREFIX_LEN..PREFIX_LEN + salt_len];
    let ct_len = be32(&blob[PREFIX_LEN + salt_len..header_len]) as usize;
    if blob.len() != header_len + ct_len + TAG_LEN || ct_len == 0 || ct_len % 16 != 0 {
        return Err(VaultError::Corrupt);
    }

    let okm = kdf.derive(passphrase, salt)?;
    let signed = &blob[..header_len + ct_len];
    let expected = mac(&okm[48..], signed)?;
    if expected.ct_eq(&blob[header_len + ct_len..]).unwrap_u8() != 1 {
        return Err(VaultError::WrongPassphrase);
    }

    let mut buf = Zeroizing::new(blob[header_len..header_len + ct_len].to_vec());
    let token = {
        use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
        type Dec = cbc::Decryptor<aes::Aes256>;
        let dec = Dec::new_from_slices(&okm[..32], &okm[32..48]).map_err(|_| VaultError::Crypto)?;
        let plain = dec
            .decrypt_padded_mut::<Pkcs7>(&mut buf)
            .map_err(|_| VaultError::Crypto)?;
        unpad_plaintext(plain)?
    };
    buf.zeroize();
    Ok(token)
}

pub fn describe(blob: &[u8]) -> Option<Kdf> {
    if blob.len() < PREFIX_LEN + 4 + TAG_LEN || &blob[..6] != MAGIC || blob[6] != VERSION {
        return None;
    }
    Some(Kdf {
        id: blob[7],
        p1: be32(&blob[8..12]),
        p2: be32(&blob[12..16]),
        p3: be32(&blob[16..20]),
    })
}

fn mac(key: &[u8], data: &[u8]) -> Result<[u8; 32], VaultError> {
    let mut h = <Hmac<Sha256> as Mac>::new_from_slice(key).map_err(|_| VaultError::Crypto)?;
    h.update(data);
    Ok(h.finalize().into_bytes().into())
}

fn be32(b: &[u8]) -> u32 {
    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
}

fn pad_plaintext(token: &str) -> Result<Zeroizing<Vec<u8>>, VaultError> {
    let raw = token.as_bytes();
    if raw.is_empty() || raw.len() > u16::MAX as usize {
        return Err(VaultError::Crypto);
    }
    let body = 2 + raw.len();
    let total = body.div_ceil(PAD_TO) * PAD_TO;
    let mut out = Zeroizing::new(vec![0u8; total]);
    out[..2].copy_from_slice(&(raw.len() as u16).to_be_bytes());
    out[2..body].copy_from_slice(raw);
    random(&mut out[body..])?;
    Ok(out)
}

fn unpad_plaintext(plain: &[u8]) -> Result<Secret, VaultError> {
    if plain.len() < 2 {
        return Err(VaultError::Corrupt);
    }
    let len = u16::from_be_bytes([plain[0], plain[1]]) as usize;
    if len == 0 || 2 + len > plain.len() {
        return Err(VaultError::Corrupt);
    }
    let text = std::str::from_utf8(&plain[2..2 + len]).map_err(|_| VaultError::Corrupt)?;
    Ok(Zeroizing::new(text.to_string()))
}


const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn b64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(B64[(n >> 18 & 63) as usize] as char);
        out.push(B64[(n >> 12 & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            B64[(n >> 6 & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

pub fn b64_decode(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    for c in text.bytes() {
        if c == b'=' || c.is_ascii_whitespace() {
            continue;
        }
        let v = B64.iter().position(|&x| x == c)? as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quick() -> Kdf {
        Kdf {
            id: KDF_ARGON2ID,
            p1: 8 * 1024,
            p2: 1,
            p3: 1,
        }
    }

    #[test]
    fn ida_y_vuelta() {
        let token = "a".repeat(200);
        let blob = seal(&token, "una passphrase larga", quick()).unwrap();
        assert_eq!(*open(&blob, "una passphrase larga").unwrap(), token);
    }

    #[test]
    fn ida_y_vuelta_pbkdf2() {
        let kdf = Kdf {
            id: KDF_PBKDF2,
            p1: 100_000,
            p2: 0,
            p3: 0,
        };
        let blob = seal("token-de-prueba", "otra passphrase larga", kdf).unwrap();
        assert_eq!(*open(&blob, "otra passphrase larga").unwrap(), "token-de-prueba");
    }

    #[test]
    fn passphrase_mala() {
        let blob = seal("token", "passphrase correcta", quick()).unwrap();
        assert!(matches!(
            open(&blob, "passphrase distinta"),
            Err(VaultError::WrongPassphrase)
        ));
    }

    #[test]
    fn criptograma_tocado() {
        let mut blob = seal("token", "passphrase correcta", quick()).unwrap();
        let last = blob.len() - 40;
        blob[last] ^= 0x01;
        assert!(matches!(
            open(&blob, "passphrase correcta"),
            Err(VaultError::WrongPassphrase)
        ));
    }

    #[test]
    fn parametros_rebajados_se_rechazan() {
        let mut blob = seal("token", "passphrase correcta", quick()).unwrap();
        blob[8..12].copy_from_slice(&1u32.to_be_bytes());
        assert!(matches!(
            open(&blob, "passphrase correcta"),
            Err(VaultError::Corrupt)
        ));
    }

    #[test]
    fn el_relleno_oculta_la_longitud() {
        let corto = seal("abc", "passphrase correcta", quick()).unwrap();
        let largo = seal(&"x".repeat(50), "passphrase correcta", quick()).unwrap();
        assert_eq!(corto.len(), largo.len());
    }

    #[test]
    fn base64_ida_y_vuelta() {
        for n in 0..40usize {
            let data: Vec<u8> = (0..n).map(|i| (i * 7 + 3) as u8).collect();
            assert_eq!(b64_decode(&b64_encode(&data)).unwrap(), data);
        }
    }
}

#[cfg(test)]
mod interop {
    use super::*;

    const VAULT_DE_BASH: &str = "SFRCSEZDAQIAAYagAAAAAAAAAACsCv+NSUWYQAAAAFABDn2M3fGu0++16otqqISXMli2i9M4sGLT2vYUrT5BGgPJ2iaK+hf2BxuNj1JH7HXSCq1/YZCzhL4l5+gbut4OoSllwRy0HoqmwOLnAg9fjwaJA/F255YojJV8EILAWfOX6voqm0R4StfZ2m11dtQ4";

    #[test]
    fn rust_abre_lo_que_cifra_bash() {
        let blob = b64_decode(VAULT_DE_BASH).expect("base64 del vault de Bash");
        let token = open(&blob, "passphrase de interoperabilidad")
            .expect("Rust deberia abrir un vault escrito por la version de Bash");
        assert_eq!(*token, "token-de-interoperabilidad");
    }

    const VAULT_DE_POWERSHELL: &str = "SFRCSEZDAQIAAYagAAAAAAAAAADCR01jGy0yKwAAAFA6H02duo9tD48ZWxToU7wwK79LynmSpApHm3Y1TPsurjAcXsBFd7J+6/t+dvCo/SeFIgL3XuNsTc/5VjLdYpA7ypfv3JC6bbOUGJw975UAquALebukCTzVm2gn2NhEirQUk8pwOa5YDLd1E8XOUOYn";

    #[test]
    fn rust_abre_lo_que_cifra_powershell() {
        let blob = b64_decode(VAULT_DE_POWERSHELL).expect("base64 del vault de PowerShell");
        let token = open(&blob, "otra passphrase larga aqui")
            .expect("Rust deberia abrir un vault escrito por la version de PowerShell");
        assert_eq!(*token, "token-escrito-por-powershell-0123456789");
    }

    #[test]
    fn el_vault_de_bash_tambien_rechaza_la_passphrase_mala() {
        let blob = b64_decode(VAULT_DE_BASH).unwrap();
        assert!(matches!(
            open(&blob, "otra passphrase cualquiera"),
            Err(VaultError::WrongPassphrase)
        ));
    }

    #[test]
    fn la_derivacion_es_la_de_openssl() {
        use sha2::Digest;
        let kdf = Kdf {
            id: KDF_PBKDF2,
            p1: 100_000,
            p2: 0,
            p3: 0,
        };
        let okm = kdf
            .derive("passphrase de interoperabilidad", &[0u8, 1, 2, 3, 4, 5, 6, 7])
            .unwrap();
        let huella: String = sha2::Sha512::digest(&okm[..])[..8]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(huella, "0acb31ce32e83fea");
    }
}
