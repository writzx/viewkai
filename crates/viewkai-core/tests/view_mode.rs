//! Integration tests for `viewkai_core::ViewMode`.

use viewkai_core::ViewMode;

#[test]
fn view_mode_serde_continuous() {
    let json = serde_json::to_string(&ViewMode::Continuous).unwrap();
    let decoded: ViewMode = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, ViewMode::Continuous);
}

#[test]
fn view_mode_serde_spread_true() {
    let json = serde_json::to_string(&ViewMode::Spread {
        cover_separate: true,
    })
    .unwrap();
    let decoded: ViewMode = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, ViewMode::Spread { cover_separate: true });
}

#[test]
fn view_mode_serde_spread_false() {
    let json = serde_json::to_string(&ViewMode::Spread {
        cover_separate: false,
    })
    .unwrap();
    let decoded: ViewMode = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, ViewMode::Spread { cover_separate: false });
}

#[test]
fn view_mode_default_is_continuous() {
    assert_eq!(ViewMode::default(), ViewMode::Continuous);
}
