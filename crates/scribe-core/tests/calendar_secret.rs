use scribe_core::calendar::Secret;

const TOKEN: &str = "0.refresh-token-value";

#[test]
fn a_secret_hides_its_value_when_printed_for_debugging() {
    let secret = Secret::new(TOKEN);

    let printed = format!("{secret:?}");

    assert_eq!(printed, "Secret(***)");
}

#[test]
fn a_secret_gives_up_its_value_only_when_exposed() {
    let secret = Secret::new(TOKEN);

    assert_eq!(secret.expose(), TOKEN);
}
