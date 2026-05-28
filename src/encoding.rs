use std::io;

/// 检测 BOM，返回 (编码名, BOM 长度)
fn detect_bom(bytes: &[u8]) -> (Option<&'static str>, usize) {
    if bytes.len() >= 4 && bytes[..4] == [0x00, 0x00, 0xFE, 0xFF] {
        (Some("utf-32be"), 4)
    } else if bytes.len() >= 4 && bytes[..4] == [0xFF, 0xFE, 0x00, 0x00] {
        (Some("utf-32le"), 4)
    } else if bytes.len() >= 3 && bytes[..3] == [0xEF, 0xBB, 0xBF] {
        (Some("utf-8"), 3)
    } else if bytes.len() >= 2 && bytes[..2] == [0xFE, 0xFF] {
        (Some("utf-16be"), 2)
    } else if bytes.len() >= 2 && bytes[..2] == [0xFF, 0xFE] {
        (Some("utf-16le"), 2)
    } else {
        (None, 0)
    }
}

/// 将字节解码为 UTF-8 String
/// - `input_encoding` = "auto" 时自动检测 BOM，无 BOM 则默认为 UTF-8
/// - 支持 utf-8 / utf-16le / utf-16be / gbk / big5 / shift-jis / euc-kr / latin-1 等
pub fn decode_bytes(bytes: &[u8], input_encoding: &str) -> io::Result<String> {
    let enc_name = if input_encoding == "auto" {
        let (bom_enc, bom_len) = detect_bom(bytes);
        // 有 BOM → 用 BOM 指定的编码，跳过 BOM 字节
        if let Some(enc) = bom_enc {
            let enc = encoding_rs::Encoding::for_label(enc.as_bytes()).unwrap();
            let (cow, had_errors) = enc.decode_without_bom_handling(&bytes[bom_len..]);
            return if had_errors {
                Err(io::Error::new(io::ErrorKind::InvalidData, format!("invalid {} data after BOM", enc.name())))
            } else {
                Ok(cow.into_owned())
            };
        }
        // 无 BOM → 尝试 UTF-8
        "utf-8"
    } else {
        input_encoding
    };

    let encoding = encoding_rs::Encoding::for_label(enc_name.as_bytes())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, format!("unknown encoding: {enc_name}")))?;

    let (cow, _, had_errors) = encoding.decode(bytes);
    if had_errors {
        Err(io::Error::new(io::ErrorKind::InvalidData, format!("invalid {enc_name} data")))
    } else {
        Ok(cow.into_owned())
    }
}

/// 将 UTF-8 文本编码为指定编码的字节
pub fn encode_text(text: &str, output_encoding: &str) -> Vec<u8> {
    let enc_name = if output_encoding.is_empty() { "utf-8" } else { output_encoding };
    let encoding = encoding_rs::Encoding::for_label(enc_name.as_bytes()).unwrap_or(encoding_rs::UTF_8);
    let (bytes, _, _) = encoding.encode(text);
    bytes.into_owned()
}
