use radar_core::application::health_check;

#[test]
fn health_check_is_deterministic_and_versioned() {
    let first = health_check();
    let second = health_check();

    assert_eq!(first.contract_version, 1);
    assert_eq!(first.status, "ok");
    assert_eq!(first.checked_at, None);
    assert_eq!(first, second);
}
