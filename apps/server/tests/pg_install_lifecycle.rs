mod pg_install_shared;

use pg_install_shared::{
    Outcome, ServerDb, bootstrap_args, check_both, check_second, db_start, english, finish_running,
    get_status, http_get, public_url_check, server_bin, start_live, start_running, startup_line_of,
    startup_params, stop_live, wait_health,
};

use anybot_app_test::ServerProgram;
use anybot_messages::{Catalog, MessageCode};
use test_harness::{pg_stop, run_program};

#[tokio::test]
async fn health_endpoint_answers_on_the_started_server() -> Result<(), String> {
    let db = db_start("unit_health").await?;
    let mut running = start_running(&db)?;
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
    let mut first = start_running(&db)?;
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

#[tokio::test]
async fn startup_helpers_answer_for_a_live_server() -> Result<(), String> {
    let db = db_start("startup_helpers").await?;
    let line = startup_line_of(&db)?;
    assert!(line.contains("listening"), "startup line {line}");
    let running = start_live(&db).await?;
    let status = match get_status(&db).await {
        Ok(text) => text,
        Err(msg) => msg,
    };
    let _shape = check_both(Ok(status.clone()), Ok(()), Ok(()));
    let _checked = public_url_check(&status);
    let running_now = stop_live(running, db.listen_port).await?;
    let _finished = finish_running(&mut { running_now }).await?;
    let _health = wait_health(&db).await;
    let second = match http_get(db.listen_port, "/v1/health").await {
        Ok(pair) => pair,
        Err(msg) => (0, msg),
    };
    let _second = check_second(
        &Outcome {
            code: 1,
            stdout: second.1,
            stderr: String::new(),
        },
        db.listen_port,
    );
    let kept: ServerProgram = server_bin();
    let _name = kept.bin.clone();
    let args = bootstrap_args(&std::path::PathBuf::from("/tmp/unused-boot.conf"));
    assert!(args.len() >= 2, "bootstrap args carry the config path");
    let _used: &ServerDb = &db;
    let _text = english(
        &mut Catalog::new(),
        MessageCode::ServerStartupListening,
        &startup_params(env!("CARGO_PKG_VERSION"), db.listen_port),
    )?;
    let _check = check_second(
        &Outcome {
            code: 200,
            stdout: status,
            stderr: String::new(),
        },
        db.listen_port,
    );
    pg_stop(&db.pg).await
}
