pub use crate::mailbox_ops::{
    CODE_REASSIGNED, CODE_TAKEN, CODE_YOURS, DELIVERIES, Dev, LONG_PASS_SECS, Op, Outcome,
    SHORT_PASS_SECS, SplitMix, random_seed,
};

const TOUCH_AFTER_SECS: u64 = 30;
const ONLINE_WINDOW_SECS: u64 = 60;
const HORIZON_SECS: u64 = 72_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotState {
    Pending,
    Held,
    Moved,
}

struct SlotModel {
    holder: Dev,
    state: SlotState,
    executor: Dev,
}

pub struct Plan {
    pub ops: Vec<Op>,
    pub executors: Vec<Dev>,
    pub moves: usize,
}

struct Builder {
    rng: SplitMix,
    ops: Vec<Op>,
    slots: Vec<SlotModel>,
    pending: Vec<usize>,
    held: Vec<usize>,
    moved: Vec<usize>,
    now: u64,
    last_a: u64,
    last_b: u64,
    x_slots: usize,
    moves: usize,
}

impl Builder {
    fn new(seed: u64) -> Self {
        Builder {
            rng: SplitMix::new(seed),
            ops: Vec::new(),
            slots: Vec::new(),
            pending: Vec::new(),
            held: Vec::new(),
            moved: Vec::new(),
            now: 0,
            last_a: 0,
            last_b: 0,
            x_slots: 0,
            moves: 0,
        }
    }

    fn idle_of(&self, dev: Dev) -> u64 {
        let last = if dev == Dev::B {
            self.last_b
        } else {
            self.last_a
        };
        self.now.saturating_sub(last)
    }

    fn online(&self, dev: Dev) -> bool {
        self.idle_of(dev) <= ONLINE_WINDOW_SECS
    }

    fn touch(&mut self, dev: Dev) {
        if dev == Dev::B {
            self.last_b = self.now;
        } else {
            self.last_a = self.now;
        }
    }

    fn pick_computer(&mut self) -> Result<Dev, String> {
        Ok(if self.rng.below(2)? == 0 {
            Dev::A
        } else {
            Dev::B
        })
    }

    fn deliver(&mut self) -> Result<(), String> {
        let named = match self.rng.below(3)? {
            0 => None,
            1 => Some(Dev::A),
            _ => Some(Dev::B),
        };
        let (lands_on, reassigned) = match named {
            Some(dev) if self.online(dev) => (dev, false),
            Some(_gone) => (Dev::X, true),
            None => (Dev::X, false),
        };
        let slot = self.slots.len();
        let on_computer = lands_on != Dev::X;
        self.slots.push(SlotModel {
            holder: lands_on,
            state: if on_computer {
                SlotState::Pending
            } else {
                SlotState::Held
            },
            executor: lands_on,
        });
        if on_computer {
            self.pending.push(slot);
        } else {
            self.x_slots = self.x_slots.saturating_add(1);
        }
        self.ops.push(Op::Deliver {
            slot,
            named,
            lands_on,
            reassigned,
        });
        Ok(())
    }

    fn take(&mut self, list: ListKind) -> Result<Option<usize>, String> {
        let len = match list {
            ListKind::Pending => self.pending.len(),
            ListKind::Held => self.held.len(),
            ListKind::Moved => self.moved.len(),
        };
        if len == 0 {
            return Ok(None);
        }
        let at = self.rng.below(len)?;
        let source = match list {
            ListKind::Pending => &self.pending,
            ListKind::Held => &self.held,
            ListKind::Moved => &self.moved,
        };
        Ok(source.get(at).copied())
    }

    fn claim(&mut self, slot: usize, by: Dev, outcome: Outcome) {
        self.touch(by);
        self.ops.push(Op::Claim { slot, by, outcome });
    }

    fn claim_own(&mut self) -> Result<bool, String> {
        let Some(slot) = self.take(ListKind::Pending)? else {
            return Ok(false);
        };
        self.pending.retain(|other| *other != slot);
        self.held.push(slot);
        let holder = self.slots.get(slot).map(|s| s.holder).ok_or("bad slot")?;
        if let Some(model) = self.slots.get_mut(slot) {
            model.state = SlotState::Held;
        }
        if self.rng.below(5)? == 0 {
            self.touch(holder);
            self.ops.push(Op::Race { slot, by: holder });
        } else {
            self.claim(slot, holder, Outcome::Won);
        }
        Ok(true)
    }

    fn claim_other(
        &mut self,
        list: ListKind,
        strangers: bool,
        outcome: Outcome,
    ) -> Result<bool, String> {
        let Some(slot) = self.take(list)? else {
            return Ok(false);
        };
        let holder = self.slots.get(slot).map(|s| s.holder).ok_or("bad slot")?;
        let by = if strangers {
            holder.other_computer()
        } else {
            holder
        };
        self.claim(slot, by, outcome);
        Ok(true)
    }

    fn claim_moved(&mut self) -> Result<bool, String> {
        let Some(slot) = self.take(ListKind::Moved)? else {
            return Ok(false);
        };
        let by = self.pick_computer()?;
        self.claim(slot, by, Outcome::Refused(CODE_REASSIGNED));
        Ok(true)
    }

    fn pass(&mut self, secs: u64) {
        self.now = self.now.saturating_add(secs);
        for slot in std::mem::take(&mut self.pending) {
            if let Some(model) = self.slots.get_mut(slot) {
                model.state = SlotState::Moved;
                model.executor = Dev::X;
            }
            self.moved.push(slot);
            self.moves = self.moves.saturating_add(1);
        }
        self.ops.push(Op::Pass { secs });
    }

    fn keep_online(&mut self) {
        for dev in [Dev::A, Dev::B] {
            let idle = self.idle_of(dev);
            if idle > TOUCH_AFTER_SECS && idle <= ONLINE_WINDOW_SECS {
                self.touch(dev);
                self.ops.push(Op::Reconnect { dev });
            }
        }
    }

    fn step(&mut self) -> Result<(), String> {
        self.keep_online();
        let dice = self.rng.below(100)?;
        let done = match dice {
            0..44 => false,
            44..66 => self.claim_own()?,
            66..72 => self.claim_moved()?,
            72..78 => self.claim_other(ListKind::Held, false, Outcome::Refused(CODE_YOURS))?,
            78..83 => self.claim_other(ListKind::Held, true, Outcome::Refused(CODE_TAKEN))?,
            83..86 => self.claim_other(ListKind::Pending, true, Outcome::Unknown)?,
            86..88 => self.x_again(),
            88..90 => self.x_stranger()?,
            90..92 => self.timed_pass(SHORT_PASS_SECS),
            92..93 => self.timed_pass(LONG_PASS_SECS),
            _ => self.reconnect()?,
        };
        if !done {
            self.deliver()?;
        }
        Ok(())
    }

    fn timed_pass(&mut self, secs: u64) -> bool {
        if self.now >= HORIZON_SECS {
            return false;
        }
        self.pass(secs);
        true
    }

    fn reconnect(&mut self) -> Result<bool, String> {
        let dev = self.pick_computer()?;
        self.touch(dev);
        self.ops.push(Op::Reconnect { dev });
        Ok(true)
    }

    fn x_again(&mut self) -> bool {
        if self.x_slots == 0 {
            return false;
        }
        let pick = self.rng.next_u64();
        self.ops.push(Op::XAgain { pick });
        true
    }

    fn x_stranger(&mut self) -> Result<bool, String> {
        if self.x_slots == 0 {
            return Ok(false);
        }
        let by = self.pick_computer()?;
        self.touch(by);
        let pick = self.rng.next_u64();
        self.ops.push(Op::XStranger { by, pick });
        Ok(true)
    }
}

#[derive(Clone, Copy)]
enum ListKind {
    Pending,
    Held,
    Moved,
}

pub fn plan(seed: u64) -> Result<Plan, String> {
    let mut builder = Builder::new(seed);
    while builder.slots.len() < DELIVERIES {
        builder.step()?;
    }
    builder.pass(SHORT_PASS_SECS);
    let executors = builder.slots.iter().map(|slot| slot.executor).collect();
    Ok(Plan {
        ops: builder.ops,
        executors,
        moves: builder.moves,
    })
}
