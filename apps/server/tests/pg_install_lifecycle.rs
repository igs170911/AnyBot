use anybot_domain as _;
use anybot_server_test as _;
mod pg_install_shared;

use anybot_app_test::{check_second, wait_health};
use pg_install_shared::{db_start, server_bin, startup_line_of};

use test_harness::{pg_stop, run_program};

fn listen_line(db: &anybot_app_test::ServerDb) -> Result<String, String> {
    startup_line_of(db)
}

#[tokio::test]
async fn health_endpoint_answers_on_the_started_server() -> Result<(), String> {
    let db = db_start("unit_health").await?;
    let mut running = server_bin().start_running(&db, &listen_line(&db)?)?;
    let ready = wait_health(&db).await;
    let result = running.kill();
    let stopped = pg_stop(&db.pg).await;
    match result {
        Ok(_outcome) if ready.is_ok() => stopped,
        Ok(_outcome) => ready,
        Err(kill) => Err(kill),
    }
}

#[tokio::test]
async fn second_server_exits_with_one_while_first_keeps_serving_health() -> Result<(), String> {
    let db = db_start("two_servers_second_fails").await?;
    let mut first = server_bin().start_running(&db, &listen_line(&db)?)?;
    let ready = wait_health(&db).await;
    let second = run_program(&server_bin().program(db.args()), &db.dir)?;
    let check = check_second(&second, db.listen_port);
    let _result = first.kill();
    let stopped = pg_stop(&db.pg).await;
    match check {
        Ok(()) if ready.is_ok() => stopped,
        Ok(()) => ready,
        Err(msg) => Err(msg),
    }
}
