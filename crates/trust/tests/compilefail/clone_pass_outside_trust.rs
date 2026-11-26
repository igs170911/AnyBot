use anybot_trust::{ActionRequest, GateConfig, run_action};

fn main() {
    let config = GateConfig::plain();
    let request: Option<ActionRequest> = None;
    let step = request.map(|r| run_action(&r, &NoComputer, &config, &NoView, &anybot_foundation::SystemClock));
    let pass = step.and_then(|s| s.pass().clone());
    println!("{pass:?}");
}

struct NoComputer;
struct NoView;
