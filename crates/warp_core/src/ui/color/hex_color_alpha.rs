//! 支持 RRGGBBAA 格式 (8 位 hex) 的 serde 序列化模块。
//! 同时兼容 RRGGBB (6 位) 格式，此时 alpha 默认为 255 (不透明)。

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use warpui::color::ColorU;

use super::OPAQUE;

const SHORT_LEN: usize = 3;
const RGB_LEN: usize = 6;
const RGBA_LEN: usize = 8;

/// 从 hex 字符串解析 ColorU，支持 #RGB、#RRGGBB、#RRGGBBAA 格式。
fn coloru_from_hex_alpha(s: &str) -> Result<ColorU, String> {
    if !s.starts_with('#') {
        return Err("Expected hex color string starting with #".to_string());
    }

    let hex = &s[1..];

    if hex.len() != SHORT_LEN && hex.len() != RGB_LEN && hex.len() != RGBA_LEN {
        return Err(format!(
            "Expected hex color string with 3, 6, or 8 characters after #, got {}",
            hex.len()
        ));
    }

    // 展开 3 位缩写: #RGB -> #RRGGBB
    let expanded: String = if hex.len() == SHORT_LEN {
        hex.chars().flat_map(|c| std::iter::repeat_n(c, 2)).collect()
    } else {
        hex.to_string()
    };

    let parsed: Result<Vec<u8>, _> = (0..expanded.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&expanded[i..i + 2], 16))
        .collect();

    match parsed {
        Ok(bytes) => match bytes.len() {
            3 => Ok(ColorU {
                r: bytes[0],
                g: bytes[1],
                b: bytes[2],
                a: OPAQUE,
            }),
            4 => Ok(ColorU {
                r: bytes[0],
                g: bytes[1],
                b: bytes[2],
                a: bytes[3],
            }),
            _ => Err("Invalid hex color length".to_string()),
        },
        Err(_) => Err("Invalid hex color string".to_string()),
    }
}

/// 将 ColorU 序列化为 hex 字符串。
/// 当 alpha 为 255 时输出 6 位 (#RRGGBB) 以保持简洁，否则输出 8 位 (#RRGGBBAA)。
fn coloru_to_hex_alpha_string(color: &ColorU) -> String {
    if color.a == OPAQUE {
        format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
    } else {
        format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            color.r, color.g, color.b, color.a
        )
    }
}

/// serde deserialize 函数，用于 `#[serde(with = "hex_color_alpha")]`。
pub fn deserialize<'de, D, C>(deserializer: D) -> Result<C, D::Error>
where
    C: From<ColorU>,
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    coloru_from_hex_alpha(&s)
        .map(Into::into)
        .map_err(de::Error::custom)
}

/// serde serialize 函数，用于 `#[serde(with = "hex_color_alpha")]`。
pub fn serialize<S, C>(color: &C, serializer: S) -> Result<S::Ok, S::Error>
where
    C: Into<ColorU> + Clone,
    S: Serializer,
{
    let coloru: ColorU = color.to_owned().into();
    coloru_to_hex_alpha_string(&coloru).serialize(serializer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_6char() {
        let c = coloru_from_hex_alpha("#3994BC").unwrap();
        assert_eq!(c.r, 0x39);
        assert_eq!(c.g, 0x94);
        assert_eq!(c.b, 0xBC);
        assert_eq!(c.a, 255);
    }

    #[test]
    fn test_parse_8char() {
        let c = coloru_from_hex_alpha("#3994BCB3").unwrap();
        assert_eq!(c.r, 0x39);
        assert_eq!(c.g, 0x94);
        assert_eq!(c.b, 0xBC);
        assert_eq!(c.a, 0xB3);
    }

    #[test]
    fn test_parse_3char() {
        let c = coloru_from_hex_alpha("#ABC").unwrap();
        assert_eq!(c.r, 0xAA);
        assert_eq!(c.g, 0xBB);
        assert_eq!(c.b, 0xCC);
        assert_eq!(c.a, 255);
    }

    #[test]
    fn test_serialize_opaque() {
        let c = ColorU { r: 0x39, g: 0x94, b: 0xBC, a: 255 };
        assert_eq!(coloru_to_hex_alpha_string(&c), "#3994bc");
    }

    #[test]
    fn test_serialize_with_alpha() {
        let c = ColorU { r: 0x39, g: 0x94, b: 0xBC, a: 0xB3 };
        assert_eq!(coloru_to_hex_alpha_string(&c), "#3994bcb3");
    }
}
