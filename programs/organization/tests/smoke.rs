use litesvm::LiteSVM;

#[test]
fn program_deploys_in_litesvm() {
    let program_id = organization::id();
    let bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/organization.so"
    ));
    let mut svm = LiteSVM::new();
    svm.add_program(program_id, bytes).unwrap();
}
