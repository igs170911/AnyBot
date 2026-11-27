#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin<'a> {
    Computer(&'a str),
    Phone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteInput<'a> {
    pub origin: Origin<'a>,
    pub team_member: bool,
    pub model_only_on: Option<&'a str>,
    pub named: Option<&'a str>,
    pub bot_computer: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target<'a> {
    Device(&'a str),
    BotComputer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteSource {
    Team,
    ModelOnly,
    Named,
    BotSetting,
    Origin,
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Route<'a> {
    pub target: Target<'a>,
    pub source: RouteSource,
}

#[must_use]
pub fn choose_route<'a>(input: &RouteInput<'a>) -> Route<'a> {
    if input.team_member {
        return bot_computer(RouteSource::Team);
    }
    let picks = [
        (input.model_only_on, RouteSource::ModelOnly),
        (input.named, RouteSource::Named),
        (input.bot_computer, RouteSource::BotSetting),
    ];
    for (device, source) in picks {
        if let Some(device) = device {
            return Route {
                target: Target::Device(device),
                source,
            };
        }
    }
    match input.origin {
        Origin::Computer(device) => Route {
            target: Target::Device(device),
            source: RouteSource::Origin,
        },
        Origin::Phone => bot_computer(RouteSource::Default),
    }
}

fn bot_computer(source: RouteSource) -> Route<'static> {
    Route {
        target: Target::BotComputer,
        source,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Availability {
    pub online: bool,
    pub delegate_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReassignReason {
    Offline,
    DelegateOff,
}

impl ReassignReason {
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            ReassignReason::Offline => "offline",
            ReassignReason::DelegateOff => "delegate-off",
        }
    }
}

#[must_use]
pub fn reassign_reason(
    route: &Route<'_>,
    origin: &Origin<'_>,
    availability: Option<Availability>,
) -> Option<ReassignReason> {
    let Target::Device(device) = route.target else {
        return None;
    };
    if route.source == RouteSource::ModelOnly {
        return None;
    }
    let Some(available) = availability else {
        return Some(ReassignReason::Offline);
    };
    if !available.online {
        return Some(ReassignReason::Offline);
    }
    let own_round = *origin == Origin::Computer(device);
    if !own_round && !available.delegate_enabled {
        return Some(ReassignReason::DelegateOff);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(origin: Origin<'_>) -> RouteInput<'_> {
        RouteInput {
            origin,
            team_member: false,
            model_only_on: None,
            named: None,
            bot_computer: None,
        }
    }

    #[test]
    fn a_round_started_on_a_computer_stays_there_and_the_phone_goes_to_the_bot() {
        let from_a = choose_route(&input(Origin::Computer("a")));
        assert_eq!(from_a.target, Target::Device("a"));
        assert_eq!(from_a.source, RouteSource::Origin);
        let from_phone = choose_route(&input(Origin::Phone));
        assert_eq!(from_phone.target, Target::BotComputer);
        assert_eq!(from_phone.source, RouteSource::Default);
    }

    #[test]
    fn the_bot_setting_beats_the_origin_and_a_named_computer_beats_the_setting() {
        let mut wanted = input(Origin::Computer("a"));
        wanted.bot_computer = Some("b");
        assert_eq!(choose_route(&wanted).target, Target::Device("b"));
        wanted.named = Some("c");
        let named = choose_route(&wanted);
        assert_eq!(named.target, Target::Device("c"));
        assert_eq!(named.source, RouteSource::Named);
    }

    #[test]
    fn a_model_only_one_computer_can_reach_beats_everything_but_the_team_rule() {
        let mut wanted = input(Origin::Computer("a"));
        wanted.named = Some("c");
        wanted.bot_computer = Some("b");
        wanted.model_only_on = Some("m");
        assert_eq!(choose_route(&wanted).target, Target::Device("m"));
        wanted.team_member = true;
        let team = choose_route(&wanted);
        assert_eq!(team.target, Target::BotComputer);
        assert_eq!(team.source, RouteSource::Team);
    }

    #[test]
    fn an_offline_or_closed_computer_is_replaced_by_the_bot_computer() {
        let route = Route {
            target: Target::Device("b"),
            source: RouteSource::BotSetting,
        };
        let from_phone = Origin::Phone;
        let offline = Availability {
            online: false,
            delegate_enabled: true,
        };
        assert_eq!(
            reassign_reason(&route, &from_phone, Some(offline)),
            Some(ReassignReason::Offline)
        );
        assert_eq!(
            reassign_reason(&route, &from_phone, None),
            Some(ReassignReason::Offline)
        );
        let closed = Availability {
            online: true,
            delegate_enabled: false,
        };
        assert_eq!(
            reassign_reason(&route, &from_phone, Some(closed)),
            Some(ReassignReason::DelegateOff)
        );
        let open = Availability {
            online: true,
            delegate_enabled: true,
        };
        assert_eq!(reassign_reason(&route, &from_phone, Some(open)), None);
    }

    #[test]
    fn a_computer_may_always_run_its_own_round_and_the_model_only_one_never_moves() {
        let own = Route {
            target: Target::Device("a"),
            source: RouteSource::Origin,
        };
        let closed = Availability {
            online: true,
            delegate_enabled: false,
        };
        assert_eq!(
            reassign_reason(&own, &Origin::Computer("a"), Some(closed)),
            None
        );
        let pinned = Route {
            target: Target::Device("m"),
            source: RouteSource::ModelOnly,
        };
        let offline = Availability {
            online: false,
            delegate_enabled: false,
        };
        assert_eq!(
            reassign_reason(&pinned, &Origin::Phone, Some(offline)),
            None
        );
    }
}
