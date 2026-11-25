pub const INVITE_ALPHABET: &[u8] = b"23456789abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ";

#[must_use]
pub fn alphabet_len() -> u64 {
    INVITE_ALPHABET.len() as u64
}

pub fn format_invite(bytes: &[u8; 8]) -> Result<String, String> {
    let (head, rest) = bytes.split_first_chunk::<4>().ok_or("no head chunk")?;
    let (tail, _left) = rest.split_first_chunk::<4>().ok_or("no tail chunk")?;
    let mut out = group_to_text(head)?;
    out.push('-');
    out.push_str(&group_to_text(tail)?);
    Ok(out)
}

fn group_to_text(group: &[u8]) -> Result<String, String> {
    let mut out = String::with_capacity(4);
    for raw in group {
        out.push(byte_to_char(*raw)?);
    }
    Ok(out)
}

fn byte_to_char(raw: u8) -> Result<char, String> {
    let remainder = u64::from(raw)
        .checked_rem(alphabet_len())
        .ok_or("no remainder")?;
    let index = usize::try_from(remainder).map_err(|e| e.to_string())?;
    let ch = INVITE_ALPHABET.get(index).ok_or("no alphabet slot")?;
    Ok(*ch as char)
}

#[must_use]
pub fn is_valid_invite(text: &str) -> bool {
    let bytes = text.as_bytes();
    let Some(head) = bytes.get(..4) else {
        return false;
    };
    let Some(dash) = bytes.get(4) else {
        return false;
    };
    let Some(tail) = bytes.get(5..) else {
        return false;
    };
    bytes.len() == 9
        && *dash == b'-'
        && head.iter().all(|slot| INVITE_ALPHABET.contains(slot))
        && tail.iter().all(|slot| INVITE_ALPHABET.contains(slot))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invite_shape_is_two_groups_of_four_with_a_dash() {
        let code = format_invite(&[0; 8]).unwrap();
        assert_eq!(code, "2222-2222");
        assert!(is_valid_invite(&code));
    }

    #[test]
    fn every_alphabet_slot_is_formattable() {
        for index in 0..INVITE_ALPHABET.len() {
            let raw = u8::try_from(index).unwrap();
            let code = format_invite(&[raw; 8]).unwrap();
            for ch in code.chars() {
                assert!(!"01lOI".contains(ch), "alphabet leak: {ch}");
            }
        }
    }

    #[test]
    fn malformed_strings_are_rejected() {
        assert!(!is_valid_invite("222-2222"));
        assert!(!is_valid_invite("22222222"));
        assert!(!is_valid_invite("2222-222I"));
        assert!(!is_valid_invite("0222-2222"));
    }
}
