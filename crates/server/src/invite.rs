use anybot_domain::format_invite;

pub fn draw_invite() -> Result<String, String> {
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;
    format_invite(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anybot_domain::is_valid_invite;

    #[test]
    fn t_server_invite_2() {
        for _round in 0..10_000 {
            let code = draw_invite().unwrap();
            assert!(is_valid_invite(&code), "bad shape: {code}");
        }
    }

    #[test]
    fn drawn_invites_follow_the_shape() {
        for _round in 0..20 {
            let code = draw_invite().unwrap();
            assert!(is_valid_invite(&code), "bad shape: {code}");
            assert_eq!(code.len(), 9);
        }
    }
}
