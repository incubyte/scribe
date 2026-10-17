use stt_parakeet::default_models_dir;

#[test]
fn keeps_the_speech_model_in_the_per_user_app_data_folder() {
    let location = default_models_dir().unwrap();

    assert!(location.starts_with(dirs::data_local_dir().unwrap()));
    assert!(location.ends_with("Scribe/models/parakeet-tdt-0.6b-v3-int8"));
}
