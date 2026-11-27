pub const DELIVERIES: usize = 10_000;
pub const SHORT_PASS_SECS: u64 = 10;
pub const LONG_PASS_SECS: u64 = 61;

pub const CODE_REASSIGNED: &str = "mailbox-claim-reassigned";
pub const CODE_TAKEN: &str = "mailbox-claim-taken";
pub const CODE_YOURS: &str = "mailbox-claim-yours";

pub fn random_seed() -> Result<u64, String> {
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes).map_err(|e| format!("seed random failed: {e}"))?;
    Ok(u64::from_le_bytes(bytes))
}

pub struct SplitMix {
    state: u64,
}

impl SplitMix {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        SplitMix { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, bound: usize) -> Result<usize, String> {
        let wide = u64::try_from(bound).map_err(|e| e.to_string())?;
        let drawn = self
            .next_u64()
            .checked_rem(wide)
            .ok_or("random bound must be positive")?;
        usize::try_from(drawn).map_err(|e| e.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dev {
    A,
    B,
    X,
}

impl Dev {
    #[must_use]
    pub fn other_computer(self) -> Dev {
        if self == Dev::A { Dev::B } else { Dev::A }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Won,
    Refused(&'static str),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    Deliver {
        slot: usize,
        named: Option<Dev>,
        lands_on: Dev,
        reassigned: bool,
    },
    Claim {
        slot: usize,
        by: Dev,
        outcome: Outcome,
    },
    Race {
        slot: usize,
        by: Dev,
    },
    XAgain {
        pick: u64,
    },
    XStranger {
        by: Dev,
        pick: u64,
    },
    Pass {
        secs: u64,
    },
    Reconnect {
        dev: Dev,
    },
}
