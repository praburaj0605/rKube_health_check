#![no_main]

use libfuzzer_sys::fuzz_target;
use kube_health_check::sanitize_error;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };

    // Must never panic.
    let _ = sanitize_error(s.to_string());

    // Structured fixtures: planted secret patterns must not survive.
    const SECRET: &str = "FUZZSECRETVALUE99";
    let planted = [
        format!("password={SECRET}"),
        format!("postgres://u:{SECRET}@host/db"),
        format!("Authorization: Bearer {SECRET}"),
        format!("Password={SECRET};"),
        format!(r#"{{"password":"{SECRET}"}}"#),
        format!("{s}password={SECRET}"),
        format!("postgres://u:{SECRET}@h/{s}"),
    ];
    for case in planted {
        let out = sanitize_error(case);
        assert!(
            !out.contains(SECRET),
            "secret survived sanitization: {out}"
        );
    }
});
