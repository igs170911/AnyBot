pub const NAME_MAX_CHARS: usize = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameError {
    Empty,
    TooLong,
}

pub fn clean_name(raw: &str) -> Result<String, NameError> {
    let replaced: String = raw
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect();
    if replaced.chars().count() > NAME_MAX_CHARS {
        return Err(NameError::TooLong);
    }
    if replaced.trim().is_empty() {
        return Err(NameError::Empty);
    }
    Ok(replaced)
}

#[must_use]
pub fn tail_for(number: usize) -> String {
    format!(" ({number})")
}

#[must_use]
pub fn name_with_smallest_tail(base: &str, taken: &[String]) -> String {
    let mut number: usize = 2;
    loop {
        let tail = tail_for(number);
        let room = NAME_MAX_CHARS.saturating_sub(tail.chars().count());
        let head: String = base.chars().take(room).collect();
        let candidate = format!("{head}{tail}");
        if !taken.contains(&candidate) {
            return candidate;
        }
        number = number.saturating_add(1);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clash {
    None,
    OnlineHolder,
    OfflineHolder,
}

#[must_use]
pub fn login_name_plan(holder: Option<bool>) -> Clash {
    match holder {
        None => Clash::None,
        Some(true) => Clash::OnlineHolder,
        Some(false) => Clash::OfflineHolder,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_characters_become_spaces() {
        assert_eq!(clean_name("a\nb").as_deref(), Ok("a b"));
        assert_eq!(clean_name("a\r\n\tb").as_deref(), Ok("a   b"));
    }

    #[test]
    fn sixty_one_characters_are_refused() {
        assert_eq!(clean_name(&"x".repeat(61)), Err(NameError::TooLong));
        assert!(clean_name(&"x".repeat(60)).is_ok());
        assert_eq!(clean_name(" \n "), Err(NameError::Empty));
    }

    #[test]
    fn smallest_free_number_is_used() {
        let taken = vec![
            "desk".to_owned(),
            "desk (2)".to_owned(),
            "desk (4)".to_owned(),
        ];
        assert_eq!(name_with_smallest_tail("desk", &taken), "desk (3)");
        assert_eq!(name_with_smallest_tail("desk", &[]), "desk (2)");
    }

    #[test]
    fn long_base_is_cut_from_the_end() {
        let base = "y".repeat(60);
        let cut = name_with_smallest_tail(&base, &[]);
        assert_eq!(cut.chars().count(), 60);
        assert!(cut.ends_with(" (2)"));
        assert!(cut.starts_with(&"y".repeat(56)));
    }

    #[test]
    fn clash_follows_the_holder_state() {
        assert_eq!(login_name_plan(None), Clash::None);
        assert_eq!(login_name_plan(Some(true)), Clash::OnlineHolder);
        assert_eq!(login_name_plan(Some(false)), Clash::OfflineHolder);
    }
}
