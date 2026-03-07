use crate::http::jwt::{JwtError, JwtSigner};
use alloc::format;
use alloc::vec::Vec;
use ed25519_compact::KeyPair;
use heapless::String;

pub type HeaplessString<const N: usize> = heapless::String<N>;

pub struct QweatherJwtSigner {
    key_id: HeaplessString<64>,
    project_id: HeaplessString<64>,
    key_pair: KeyPair,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JwtSignerError {
    InvalidConfig,
    EncodingError,
}

impl QweatherJwtSigner {
    pub fn new(
        key_id: &str,
        project_id: &str,
        private_key_pem: &str,
    ) -> Result<Self, JwtSignerError> {
        let key_id: HeaplessString<64> =
            String::try_from(key_id).map_err(|_| JwtSignerError::InvalidConfig)?;
        let project_id: HeaplessString<64> =
            String::try_from(project_id).map_err(|_| JwtSignerError::InvalidConfig)?;

        // 移除可能存在的引号和 PEM 头部
        let private_key_pem = private_key_pem
            .trim_matches('"')
            .replace("-----BEGIN PRIVATE KEY-----", "")
            .replace("-----END PRIVATE KEY-----", "")
            .replace('\n', "")
            .replace('\r', "");

        let private_key_bytes =
            base64_decode(&private_key_pem).map_err(|_| JwtSignerError::InvalidConfig)?;

        // 处理不同的私钥格式
        // ed25519-compact 的 KeyPair::from_slice 需要 64 字节密钥对
        // 或者使用 Seed::from_slice + KeyPair::from_seed
        let key_pair = if private_key_bytes.len() == 32 {
            // 32 字节种子
            let seed = ed25519_compact::Seed::from_slice(&private_key_bytes)
                .map_err(|_| JwtSignerError::InvalidConfig)?;
            KeyPair::from_seed(seed)
        } else if private_key_bytes.len() == 64 {
            // 64 字节密钥对 (seed || public_key)
            KeyPair::from_slice(&private_key_bytes).map_err(|_| JwtSignerError::InvalidConfig)?
        } else if private_key_bytes.len() == 48 {
            // PKCS#8 格式 (48 字节)
            // 结构: SEQUENCE(2) + version(3) + oid(7) + octet_wrapper(4) + seed(32)
            // 种子从第 16 字节开始 (索引 16-47)
            let seed = ed25519_compact::Seed::from_slice(&private_key_bytes[16..48])
                .map_err(|_| JwtSignerError::InvalidConfig)?;
            KeyPair::from_seed(seed)
        } else if private_key_bytes.len() >= 32 {
            // 其他长度，尝试提取最后 32 字节
            let len = private_key_bytes.len();
            let seed = ed25519_compact::Seed::from_slice(&private_key_bytes[len - 32..])
                .map_err(|_| JwtSignerError::InvalidConfig)?;
            KeyPair::from_seed(seed)
        } else {
            return Err(JwtSignerError::InvalidConfig);
        };

        Ok(Self {
            key_id,
            project_id,
            key_pair,
        })
    }
}

impl JwtSigner for QweatherJwtSigner {
    fn sign_with_time(
        &self,
        _payload: &str,
        timestamp_secs: i64,
    ) -> Result<HeaplessString<256>, JwtError> {
        let now = timestamp_secs;

        let iat = now - 30;
        let exp = now + 900;

        let header = format!(r#"{{"alg":"EdDSA","kid":"{}"}}"#, self.key_id.as_str());
        let payload = format!(
            r#"{{"sub":"{}","iat":{},"exp":{}}}"#,
            self.project_id.as_str(),
            iat,
            exp
        );

        let header_encoded = base64url_encode(header.as_bytes());
        let payload_encoded = base64url_encode(payload.as_bytes());

        let message = format!("{}.{}", header_encoded, payload_encoded);

        let signature = self.key_pair.sk.sign(message.as_bytes(), None);

        let signature_bytes: &[u8] = signature.as_ref();
        let signature_encoded = base64url_encode(signature_bytes);

        let token = format!(
            "{}.{}.{}",
            header_encoded, payload_encoded, signature_encoded
        );

        let result: HeaplessString<256> =
            String::try_from(token.as_str()).map_err(|_| JwtError::EncodingError)?;

        Ok(result)
    }

    fn verify(&self, _token: &str) -> Result<(), JwtError> {
        Err(JwtError::InvalidSignature)
    }
}

fn base64url_encode(input: &[u8]) -> HeaplessString<128> {
    const ENCODE_TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

    let mut result: HeaplessString<128> = HeaplessString::new();
    let mut i = 0;

    while i < input.len() {
        let b0 = input[i] as usize;
        let b1 = if i + 1 < input.len() {
            input[i + 1] as usize
        } else {
            0
        };
        let b2 = if i + 2 < input.len() {
            input[i + 2] as usize
        } else {
            0
        };

        result.push(ENCODE_TABLE[b0 >> 2] as char).ok();

        if i + 1 < input.len() {
            result
                .push(ENCODE_TABLE[((b0 & 0x03) << 4) | (b1 >> 4)] as char)
                .ok();
        } else {
            result.push('=' as char).ok();
        }

        if i + 2 < input.len() {
            result
                .push(ENCODE_TABLE[((b1 & 0x0f) << 2) | (b2 >> 6)] as char)
                .ok();
            result.push(ENCODE_TABLE[b2 & 0x3f] as char).ok();
        } else if i + 1 < input.len() {
            result
                .push(ENCODE_TABLE[((b1 & 0x0f) << 2) as usize] as char)
                .ok();
            result.push('=' as char).ok();
        }

        i += 3;
    }

    result
}

fn base64_decode(input: &str) -> Result<Vec<u8>, ()> {
    const DECODE_TABLE: [i8; 128] = [
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 62, -1, -1,
        -1, 63, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, -1, -1, -1, -1, -1, -1, -1, 0, 1, 2, 3, 4,
        5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, -1, -1, -1,
        -1, -1, -1, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        46, 47, 48, 49, 50, 51, -1, -1, -1, -1, -1,
    ];

    let mut output = Vec::new();
    let mut buffer: u32 = 0;
    let mut bits_collected = 0;

    for byte in input.bytes() {
        if byte >= 128 {
            return Err(());
        }
        let value = DECODE_TABLE[byte as usize];
        if value < 0 {
            if byte == b'=' {
                break;
            }
            continue;
        }

        buffer = (buffer << 6) | (value as u32);
        bits_collected += 6;

        if bits_collected >= 8 {
            bits_collected -= 8;
            output.push((buffer >> bits_collected) as u8);
            buffer &= (1 << bits_collected) - 1;
        }
    }

    Ok(output)
}
