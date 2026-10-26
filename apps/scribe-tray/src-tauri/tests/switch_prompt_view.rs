use scribe_tray::calendar::switch_state::SwitchPromptView;

#[test]
fn the_prompt_names_both_meetings_for_the_popover() {
    let view = SwitchPromptView::new("DXC Review", "Sprint Retro");

    let json = serde_json::to_value(&view).unwrap();

    assert_eq!(
        json,
        serde_json::json!({ "from_subject": "DXC Review", "to_subject": "Sprint Retro" })
    );
}
