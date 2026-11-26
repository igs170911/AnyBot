use anybot_foundation::{BotId, TimestampMs};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddSource {
    UserCardPress,
    WebContent,
    ChannelMessage,
    BotReply,
}

impl AddSource {
    #[must_use]
    pub fn is_user_press(self) -> bool {
        matches!(self, AddSource::UserCardPress)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlwaysAddEvent {
    pub source: AddSource,
    pub action: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlwaysEntry {
    pub action: String,
    pub added_at: TimestampMs,
    pub from_bot: BotId,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AlwaysList {
    pub entries: Vec<AlwaysEntry>,
}

impl AlwaysList {
    pub fn add(&mut self, entry: AlwaysEntry) {
        self.entries.push(entry);
    }

    #[must_use]
    pub fn contains(&self, action: &str) -> bool {
        self.entries.iter().any(|e| e.action == action)
    }

    pub fn remove(&mut self, action: &str) {
        self.entries.retain(|e| e.action != action);
    }

    pub fn apply_events(
        &mut self,
        events: &[AlwaysAddEvent],
        clock: &dyn anybot_foundation::Clock,
    ) {
        for event in events {
            if !event.source.is_user_press() {
                continue;
            }
            if self.contains(&event.action) {
                continue;
            }
            let entry = AlwaysEntry {
                action: event.action.clone(),
                added_at: clock.now_ms(),
                from_bot: BotId::generate(clock),
            };
            self.add(entry);
        }
    }
}

#[must_use]
pub fn parse_untrusted_adds(text: &str) -> Vec<AlwaysAddEvent> {
    let lower = text.to_lowercase();
    let marker = "always approve";
    let mut out = vec![];
    let mut rest = lower.as_str();
    while let Some((_before, tail)) = rest.split_once(marker) {
        let head = tail.trim_start();
        let cut = head
            .char_indices()
            .find(|(_, c)| *c == '\n' || *c == '.' || *c == ';')
            .map(|(n, _)| n);
        let item = match cut {
            Some(n) => head.get(..n),
            None => Some(head),
        }
        .unwrap_or_default();
        let item = item.trim();
        let item = item.trim_start_matches([' ', ':']).trim_end();
        if !item.is_empty() {
            out.push(AlwaysAddEvent {
                source: AddSource::WebContent,
                action: format!("{marker} {item}"),
            });
        }
        rest = tail;
    }
    out
}
