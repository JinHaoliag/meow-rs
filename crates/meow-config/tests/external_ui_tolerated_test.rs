//! A config written for mihomo may still name `external-ui*`. RClash removed
//! dashboard hosting, so those keys must be **accepted and ignored** — never a
//! hard error: an existing config has to keep loading unchanged.
//!
//! Regression for the panel removal (see `plans/0001-raw-requirements.md`, D3):
//! `RawConfig` deliberately does not carry `deny_unknown_fields`.

#[test]
fn external_ui_keys_load_and_are_ignored() {
    let yaml = r#"
mixed-port: 7890
external-controller: 127.0.0.1:9090
secret: ""
external-ui: ./ui
external-ui-name: dist
external-ui-url: "https://github.com/MetaCubeX/metacubexd/archive/refs/heads/gh-pages.zip"
proxies: []
rules:
  - MATCH,DIRECT
"#;

    let raw = meow_config::parse_raw_yaml(yaml)
        .expect("a config naming external-ui* must still load (keys are ignored)");

    // The keys carry no state any more: they are not represented at all, and
    // the rest of the document parses exactly as before.
    assert_eq!(raw.mixed_port, Some(7890));
    assert_eq!(raw.external_controller.as_deref(), Some("127.0.0.1:9090"));
    assert_eq!(raw.rules.as_deref(), Some(&["MATCH,DIRECT".to_string()][..]));
}

#[test]
fn external_ui_keys_do_not_break_a_minimal_config() {
    let yaml = r#"
external-ui: ./somewhere-else
rules:
  - MATCH,DIRECT
"#;

    let raw = meow_config::parse_raw_yaml(yaml).expect("ignored keys never fail a load");
    assert!(raw.external_controller.is_none());
    assert_eq!(raw.rules.as_deref(), Some(&["MATCH,DIRECT".to_string()][..]));
}
