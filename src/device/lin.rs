//! PC <-> STM32 行协议（代号 LIN）—— 上位机侧
//!
//! 与 STM32 `src/protocol/lin.rs` 一一对应：
//!
//! ```text
//! $BODY*HH\n     HH = BODY 各字节算术和 & 0xFF，两位大写十六进制
//! ```

const HEX: [u8; 16] = *b"0123456789ABCDEF";

/// BODY -> `$BODY*HH\n`
pub fn encode(body: &str) -> Vec<u8> {
    let b = body.as_bytes();
    let sum = b.iter().fold(0u8, |a, &x| a.wrapping_add(x));
    let mut v = Vec::with_capacity(b.len() + 5);
    v.push(b'$');
    v.extend_from_slice(b);
    v.push(b'*');
    v.push(HEX[(sum >> 4) as usize]);
    v.push(HEX[(sum & 0x0F) as usize]);
    v.push(b'\n');
    v
}

fn hexval(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// 从接收缓冲里取出所有**完整且校验通过**的行，返回 BODY 文本
pub fn drain(buf: &mut Vec<u8>) -> Vec<String> {
    let mut out = Vec::new();
    loop {
        // 丢掉第一个 '$' 之前的垃圾字节
        match buf.iter().position(|&c| c == b'$') {
            Some(0) => {}
            Some(p) => {
                buf.drain(..p);
            }
            None => {
                buf.clear();
                break;
            }
        }

        let nl = match buf.iter().position(|&c| c == b'\n') {
            Some(i) => i,
            None => break,
        };
        if nl + 1 > buf.len() {
            break;
        }

        let line: Vec<u8> = buf.drain(..=nl).collect();
        if let Some(body) = verify(&line) {
            out.push(String::from_utf8_lossy(&body).to_string());
        }
    }
    out
}

/// 校验 `$BODY*HH\n`
fn verify(line: &[u8]) -> Option<Vec<u8>> {
    if line.len() < 5 || line[0] != b'$' || *line.last()? != b'\n' {
        return None;
    }
    let s = &line[..line.len() - 1];
    let star = s.iter().position(|&c| c == b'*')?;
    if s.len() != star + 3 {
        return None;
    }
    let body = &s[1..star];
    let want = (hexval(s[star + 1])? << 4) | hexval(s[star + 2])?;
    let sum = body.iter().fold(0u8, |a, &x| a.wrapping_add(x));
    if sum != want {
        return None;
    }
    Some(body.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let f = encode("MODE DR");
        assert_eq!(f[0], b'$');
        assert_eq!(*f.last().unwrap(), b'\n');
        let mut buf = f;
        let got = drain(&mut buf);
        assert_eq!(got, vec!["MODE DR".to_string()]);
        assert!(buf.is_empty());
    }

    #[test]
    fn drops_garbage_and_bad_checksum() {
        let mut buf = b"junk".to_vec();
        buf.extend_from_slice(&encode("PING"));
        // 篡改校验位
        let mut bad = encode("SET KV 120");
        let n = bad.len();
        bad[n - 2] = b'0';
        bad[n - 3] = b'0';
        buf.extend_from_slice(&bad);
        let got = drain(&mut buf);
        assert_eq!(got, vec!["PING".to_string()]);
    }
}
