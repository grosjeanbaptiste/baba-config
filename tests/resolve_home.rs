//! `resolve_data_file` falls back to the canonical checkout under `$HOME`.
//! Its own test binary: it mutates process-wide env vars.

use baba_config::resolve_data_file;

#[test]
fn data_file_falls_back_to_home_source_baba() {
    let home = std::env::temp_dir().join(format!("baba-config-home-{}", std::process::id()));
    let data = home.join("Source/Baba/corpus/data");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::write(data.join("probe.pl"), "% probe").unwrap();

    // SAFETY: single-threaded — this binary holds one test.
    unsafe {
        std::env::set_var("HOME", &home);
        std::env::remove_var("XDG_CONFIG_HOME");
        std::env::remove_var("BABA_DATA_DIR");
        std::env::remove_var("BABA_PROBE");
    }
    let got = resolve_data_file("BABA_PROBE", "probe.pl");
    let _ = std::fs::remove_dir_all(&home);
    assert_eq!(got, Some(data.join("probe.pl")));
}
