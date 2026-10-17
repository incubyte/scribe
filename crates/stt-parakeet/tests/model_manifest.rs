use stt_parakeet::PARAKEET_MODEL;

fn pinned_revision(url: &str) -> &str {
    let after_resolve = url.split("/resolve/").nth(1).unwrap();
    after_resolve.split('/').next().unwrap()
}

#[test]
fn every_model_file_is_fetched_over_https() {
    for file in PARAKEET_MODEL {
        assert!(file.url.starts_with("https://"), "{}", file.name);
    }
}

#[test]
fn every_model_file_is_pinned_to_a_full_commit() {
    for file in PARAKEET_MODEL {
        let revision = pinned_revision(file.url);
        assert_eq!(revision.len(), 40, "{}", file.name);
        assert!(
            revision.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')),
            "{}",
            file.name
        );
    }
}

#[test]
fn every_model_file_has_a_lowercase_sha256() {
    for file in PARAKEET_MODEL {
        assert_eq!(file.sha256.len(), 64, "{}", file.name);
        assert!(
            file.sha256
                .chars()
                .all(|c| matches!(c, '0'..='9' | 'a'..='f')),
            "{}",
            file.name
        );
    }
}

#[test]
fn every_model_file_has_a_distinct_local_name_and_a_size() {
    let mut names: Vec<&str> = PARAKEET_MODEL.iter().map(|file| file.name).collect();
    names.sort_unstable();
    names.dedup();

    assert_eq!(names.len(), PARAKEET_MODEL.len());
    assert!(PARAKEET_MODEL.iter().all(|file| file.bytes > 0));
}
