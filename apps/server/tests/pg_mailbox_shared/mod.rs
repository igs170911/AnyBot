use serde_json::json;

use anybot_app_test::mailbox_prop::{Seat, Seats, make_bot};
use anybot_app_test::serve::Served;
use anybot_server_test::computers_support::{api_post, computer_login};
use anybot_server_test::make_admin;
use anybot_store_server::runner_device;

pub(crate) async fn runner_seat(served: &Served) -> Result<Seat, String> {
    let keys = served.db.secret_keys()?;
    let opened = runner_device::open_key(&served.pool, &keys)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("no runner device")?;
    let row = runner_device::find(&served.pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("no runner device")?;
    Ok(Seat {
        bearer: opened.expose().to_owned(),
        device_id: row.device_id,
    })
}

pub(crate) async fn enable_delegate(served: &Served, seat: &Seat, key: &str) -> Result<(), String> {
    let body = json!({"device_id": seat.device_id, "enabled": true});
    let answer = api_post(served, &seat.bearer, "/v1/computers/delegate", &body, key).await?;
    if answer.status == 200 {
        return Ok(());
    }
    Err(format!("delegate refused: {}", answer.json))
}

pub(crate) async fn three_stand_ins(served: &Served, bot: &str) -> Result<Seats, String> {
    let phone = make_admin(served).await?;
    let a = computer_login(served, "A", "k-a").await?;
    let b = computer_login(served, "B", "k-b").await?;
    let a = Seat {
        bearer: a.bearer,
        device_id: a.device_id,
    };
    let b = Seat {
        bearer: b.bearer,
        device_id: b.device_id,
    };
    enable_delegate(served, &a, "k-del-a").await?;
    enable_delegate(served, &b, "k-del-b").await?;
    make_bot(served, &phone, bot, "k-bot").await?;
    let x = runner_seat(served).await?;
    Ok(Seats {
        phone,
        a,
        b,
        x,
        bot: bot.to_owned(),
    })
}
