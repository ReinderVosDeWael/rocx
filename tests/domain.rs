use rv_docx_rs::{Points, Rule};
#[test]
fn physical_units_agree_on_an_inch_and_signed_offsets() {
    let inch = Points::new(72.).unwrap();
    assert_eq!(inch.twips().unwrap().get(), 1440);
    assert_eq!(inch.emu().unwrap().get(), 914400);
    assert_eq!(inch.half_points().unwrap().get(), 144);
    assert_eq!(Points::new(-18.).unwrap().twips().unwrap().get(), -360);
    assert_eq!(
        Points::new(f64::INFINITY).unwrap_err().rule,
        Rule::InvalidMeasurement
    );
    assert!(Points::new(f64::MAX).unwrap().emu().is_err());
}
#[test]
fn document_domain_compiles_without_external_libraries_or_other_modules() {
    let output = std::env::temp_dir().join(format!("rv-docx-domain-{}.rlib", std::process::id()));
    let result = std::process::Command::new("rustc")
        .args([
            "--edition=2024",
            "--crate-type=lib",
            "--crate-name=domain_boundary",
        ])
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/src/document/mod.rs"))
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap();
    let _ = std::fs::remove_file(output);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}
