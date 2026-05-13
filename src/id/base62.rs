use thiserror::Error;

pub const BASE62_ALPHABET: &[u8; 62] =
    b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

#[derive(Debug, Error, Eq, PartialEq)]
pub enum Base62Error {
    #[error("input contains an invalid base62 character: `{0}`")]
    InvalidCharacter(char),
}

pub fn encode_base62(mut value: u64) -> String {
    if value == 0 {
        return "0".to_owned();
    }

    let mut encoded = Vec::new();

    while value > 0 {
        let remainder = (value % 62) as usize;
        encoded.push(BASE62_ALPHABET[remainder] as char);
        value /= 62;
    }

    encoded.iter().rev().collect()
}

pub fn decode_base62(input: &str) -> Result<u64, Base62Error> {
    input.chars().try_fold(0_u64, |acc, character| {
        let value = match character {
            '0'..='9' => character as u64 - '0' as u64,
            'a'..='z' => character as u64 - 'a' as u64 + 10,
            'A'..='Z' => character as u64 - 'A' as u64 + 36,
            _ => return Err(Base62Error::InvalidCharacter(character)),
        };

        Ok(acc * 62 + value)
    })
}

#[cfg(test)]
mod tests {
    use super::{Base62Error, decode_base62, encode_base62};

    #[test]
    fn encodes_known_values() {
        assert_eq!(encode_base62(0), "0");
        assert_eq!(encode_base62(61), "Z");
        assert_eq!(encode_base62(62), "10");
        assert_eq!(encode_base62(11157), "2TX");
        assert_eq!(encode_base62(2009215674938), "zn9edcu");
    }

    #[test]
    fn decodes_known_values() {
        assert_eq!(decode_base62("0"), Ok(0));
        assert_eq!(decode_base62("Z"), Ok(61));
        assert_eq!(decode_base62("10"), Ok(62));
        assert_eq!(decode_base62("2TX"), Ok(11157));
        assert_eq!(decode_base62("zn9edcu"), Ok(2009215674938));
    }

    #[test]
    fn round_trips_edge_cases() {
        let values = [0_u64, 1, 9, 10, 61, 62, 63, 999, u32::MAX as u64, u64::MAX];

        for value in values {
            let encoded = encode_base62(value);
            let decoded = decode_base62(&encoded).expect("valid base62");
            assert_eq!(decoded, value);
        }
    }

    #[test]
    fn rejects_invalid_characters() {
        assert_eq!(
            decode_base62("hello!"),
            Err(Base62Error::InvalidCharacter('!'))
        );
    }
}
