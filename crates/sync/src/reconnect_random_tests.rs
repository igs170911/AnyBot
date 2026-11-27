use super::*;

#[test]
fn seeded_sequences_repeat_for_the_same_seed_and_differ_between_seeds() {
    let a = SeededRandom::new(5);
    let b = SeededRandom::new(5);
    let c = SeededRandom::new(6);
    let first: Vec<u64> = (0..8).map(|_n| a.next_u64().unwrap()).collect();
    let again: Vec<u64> = (0..8).map(|_n| b.next_u64().unwrap()).collect();
    let other: Vec<u64> = (0..8).map(|_n| c.next_u64().unwrap()).collect();
    assert_eq!(first, again);
    assert_ne!(first, other);
}

#[test]
fn jitter_stays_inside_half_to_two_seconds_and_covers_the_range() {
    let random = SeededRandom::new(11);
    let mut low = u64::MAX;
    let mut high = 0;
    for _n in 0..5_000 {
        let value = jitter_ms(&random).unwrap();
        assert!((500..=2_000).contains(&value), "jitter {value}");
        low = low.min(value);
        high = high.max(value);
    }
    assert!(low < 520, "lowest {low}");
    assert!(high > 1_980, "highest {high}");
}

#[test]
fn the_operating_system_source_gives_values() {
    let random = OsRandom;
    let first = random.next_u64().unwrap();
    let second = random.next_u64().unwrap();
    assert_ne!(first, second);
}
