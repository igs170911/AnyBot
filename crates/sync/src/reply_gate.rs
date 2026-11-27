use anybot_protocol::ProblemJson;

use crate::core::Core;
use crate::status::ConnectionState;
use crate::wire::WireReply;

const VERSION_OLD_CODE: &str = "install-version-old";

pub(crate) enum Gate {
    Fine,
    Halted,
    Refused(ProblemJson),
    Trouble,
}

pub(crate) fn gate(core: &Core, reply: &WireReply) -> Gate {
    if reply.status == 200 {
        return Gate::Fine;
    }
    if reply.status == 401 {
        core.halt(ConnectionState::NeedLogin);
        return Gate::Halted;
    }
    let Ok(problem) = serde_json::from_slice::<ProblemJson>(&reply.body) else {
        core.note_server_trouble();
        return Gate::Trouble;
    };
    if problem.code == VERSION_OLD_CODE {
        core.halt(ConnectionState::VersionTooOld);
        return Gate::Halted;
    }
    let refused = (400..500).contains(&reply.status) && reply.status != 408 && reply.status != 429;
    if refused {
        Gate::Refused(problem)
    } else {
        core.note_server_trouble();
        Gate::Trouble
    }
}
