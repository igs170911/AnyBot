use std::collections::{BTreeSet, HashMap};

use sqlx::Row as _;

use crate::mailbox_model::{Dev, Plan};
use crate::mailbox_prop::Rig;

struct Row {
    state: String,
    claimed_by: Option<String>,
    reassigned_to: Option<String>,
}

async fn round_rows(rig: &Rig<'_>) -> Result<HashMap<String, Row>, String> {
    let rows = sqlx::query(
        "SELECT letter_id, state, claimed_by, reassigned_to FROM mailbox_letters WHERE kind = 'round'",
    )
    .fetch_all(&rig.served.pool)
    .await
    .map_err(|e| e.to_string())?;
    let mut found = HashMap::new();
    for row in rows {
        let id: String = row.try_get("letter_id").map_err(|e| e.to_string())?;
        let state = row.try_get("state").map_err(|e| e.to_string())?;
        let claimed_by = row.try_get("claimed_by").map_err(|e| e.to_string())?;
        let reassigned_to = row.try_get("reassigned_to").map_err(|e| e.to_string())?;
        found.insert(
            id,
            Row {
                state,
                claimed_by,
                reassigned_to,
            },
        );
    }
    Ok(found)
}

fn executor_of(rows: &HashMap<String, Row>, first: &str) -> Result<String, String> {
    let mut at = first.to_owned();
    for _hop in 0..4 {
        let row = rows
            .get(&at)
            .ok_or_else(|| format!("letter {at} is gone"))?;
        match (row.state.as_str(), &row.claimed_by, &row.reassigned_to) {
            ("claimed", Some(device), _) => return Ok(device.clone()),
            ("reassigned", _, Some(next)) => at.clone_from(next),
            (other, _, _) => return Err(format!("letter {at} ended in state {other}")),
        }
    }
    Err(format!("letter {first} was reassigned too many times"))
}

fn device_of(rig: &Rig<'_>, dev: Dev) -> String {
    match dev {
        Dev::A => rig.seats.a.device_id.clone(),
        Dev::B => rig.seats.b.device_id.clone(),
        Dev::X => rig.seats.x.device_id.clone(),
    }
}

pub async fn audit(rig: &Rig<'_>, plan: &Plan) -> Result<(), String> {
    let rows = round_rows(rig).await?;
    let slots = plan.executors.len();
    if rig.ids.len() != slots {
        return Err(format!(
            "{} letters handed out for {slots} slots",
            rig.ids.len()
        ));
    }
    let wanted_rows = slots.saturating_add(plan.moves);
    if rows.len() != wanted_rows {
        return Err(format!(
            "{} round letters in the database, expected {}",
            rows.len(),
            wanted_rows
        ));
    }
    let claimed_rows = rows.values().filter(|row| row.state == "claimed").count();
    if claimed_rows != slots {
        return Err(format!("{claimed_rows} letters claimed for {slots} slots"));
    }
    for (slot, (first, dev)) in rig.ids.iter().zip(&plan.executors).enumerate() {
        let ran_on = executor_of(&rows, first)?;
        let wanted = device_of(rig, *dev);
        if ran_on != wanted {
            return Err(format!(
                "slot {slot} ran on {ran_on}, expected {wanted} ({dev:?})"
            ));
        }
    }
    audit_x(rig, plan)
}

fn audit_x(rig: &Rig<'_>, plan: &Plan) -> Result<(), String> {
    let seen = rig.x.snapshot()?;
    if !seen.refused.is_empty() || !seen.failures.is_empty() {
        return Err(format!(
            "X was refused {:?} or failed {:?}",
            seen.refused, seen.failures
        ));
    }
    let claimed = seen.claimed;
    let unique: BTreeSet<&String> = claimed.iter().collect();
    let wanted = plan.executors.iter().filter(|dev| **dev == Dev::X).count();
    if unique.len() != claimed.len() || claimed.len() != wanted {
        return Err(format!(
            "X claimed {} letters ({} distinct), expected {wanted}",
            claimed.len(),
            unique.len()
        ));
    }
    Ok(())
}
