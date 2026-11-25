#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamItem {
    pub seq: u64,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamDelta {
    pub from_seq: u64,
    pub items: Vec<StreamItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Replica {
    pub items: Vec<StreamItem>,
    pub through: u64,
    pub gaps: Vec<u64>,
}

impl Replica {
    pub fn apply(&mut self, delta: &StreamDelta) {
        let mut added: std::collections::BTreeMap<u64, StreamItem> =
            std::collections::BTreeMap::new();
        for item in &delta.items {
            if item.seq > delta.from_seq {
                added.insert(item.seq, item.clone());
            }
        }
        for item in added.into_values() {
            if self.items.iter().any(|e| e.seq == item.seq) {
                continue;
            }
            self.items.push(item);
        }
        self.items.sort_by_key(|e| e.seq);
        self.through = self.items.last().map_or(self.through, |e| e.seq);
        self.refresh_gaps();
    }

    fn refresh_gaps(&mut self) {
        self.gaps.clear();
        let mut expect = 1u64;
        for item in &self.items {
            while expect < item.seq {
                self.gaps.push(expect);
                expect = expect.saturating_add(1);
            }
            expect = expect.saturating_add(1);
        }
    }

    #[must_use]
    pub fn resync_from(&self) -> Option<u64> {
        self.gaps.first().map(|g| g.saturating_sub(1))
    }

    #[must_use]
    pub fn wants_after(&self) -> u64 {
        self.through
    }
}

#[must_use]
pub fn delta_from(server: &[StreamItem], through: u64) -> StreamDelta {
    let items: Vec<StreamItem> = server.iter().filter(|i| i.seq > through).cloned().collect();
    StreamDelta {
        from_seq: through,
        items,
    }
}
