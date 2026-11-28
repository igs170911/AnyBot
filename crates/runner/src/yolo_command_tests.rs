use crate::yolo_command::{YoloArg, parse_command, parse_yolo_arg};

#[test]
fn t_autonomy_yolo_5() {
    let on = ["", "on", "\u{958b}", "\u{6253}\u{958b}", "\u{958b}\u{555f}"];
    let off = [
        "off",
        "stop",
        "\u{95dc}",
        "\u{95dc}\u{9589}",
        "\u{95dc}\u{6389}",
        "\u{53d6}\u{6d88}",
        "\u{505c}\u{6b62}",
    ];
    let status = ["maybe", "status"];
    assert!(on.iter().all(|word| parse_yolo_arg(word) == YoloArg::On));
    assert!(off.iter().all(|word| parse_yolo_arg(word) == YoloArg::Off));
    assert!(
        status
            .iter()
            .all(|word| parse_yolo_arg(word) == YoloArg::Status)
    );
}

#[test]
fn command_names_and_spacing_are_recognised() {
    assert_eq!(parse_command("/always-approve"), Some(YoloArg::On));
    assert_eq!(parse_command("  /yolo   off "), Some(YoloArg::Off));
    assert_eq!(parse_command("/yolo status"), Some(YoloArg::Status));
    assert_eq!(parse_command("/yolo-not"), None);
    assert_eq!(parse_command("please /yolo"), None);
}
