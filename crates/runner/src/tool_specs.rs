use anybot_messages::{Catalog, Lang, MessageCode, Params};
use anybot_models::ToolSpec;
use anybot_protocol::ComputerCaps;

use crate::error::TurnError;

pub(crate) fn turn_tools(lang: Lang, caps: &ComputerCaps) -> Result<Vec<ToolSpec>, TurnError> {
    let mut catalog = Catalog::new();
    let mut specs = Vec::new();
    for def in anybot_tools::offered_tools(caps) {
        let Some(code) = MessageCode::from_code(def.description_code) else {
            return Err(TurnError::UnknownMessageCode {
                code: def.description_code.to_owned(),
            });
        };
        let description = catalog.format(lang, &code, &Params::new())?;
        specs.push(ToolSpec {
            name: def.name.to_owned(),
            description,
            parameters: def.parameters,
        });
    }
    Ok(specs)
}
