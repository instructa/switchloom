#[test]
fn product_module_dependencies_are_acyclic_and_routing_owned() {
    const MODULES: &[&str] = &[
        "config",
        "contracts",
        "error",
        "evidence",
        "hosts",
        "integrations",
        "lifecycle",
        "registry",
        "routing",
    ];
    const SOURCES: &[(&str, &str, &[&str])] = &[
        ("contracts", include_str!("../contracts.rs"), &[]),
        (
            "registry",
            include_str!("../registry.rs"),
            &["contracts", "error"],
        ),
        (
            "evidence",
            include_str!("../evidence.rs"),
            &["contracts", "error", "registry"],
        ),
        (
            "hosts",
            include_str!("../hosts.rs"),
            &["contracts", "error", "evidence"],
        ),
        (
            "config",
            include_str!("../config.rs"),
            &["contracts", "error", "hosts"],
        ),
        (
            "integrations",
            include_str!("../integrations.rs"),
            &["contracts"],
        ),
        (
            "routing",
            include_str!("../routing.rs"),
            &[
                "config",
                "contracts",
                "error",
                "evidence",
                "hosts",
                "integrations",
                "registry",
            ],
        ),
        (
            "lifecycle",
            include_str!("../lifecycle.rs"),
            &["config", "contracts", "error", "registry", "routing"],
        ),
    ];

    for (owner, source, allowed_dependencies) in SOURCES {
        assert!(
            !source.contains("anyhow"),
            "{owner} must return the concrete product error, not anyhow"
        );
        for line in source.lines().filter(|line| line.starts_with("use crate")) {
            for module in MODULES {
                if line.contains(&format!("{module}::")) {
                    assert!(
                        allowed_dependencies.contains(module),
                        "{owner} must not depend on {module}: {line}"
                    );
                }
            }
        }
    }

    let hosts = include_str!("../hosts.rs");
    let routing = include_str!("../routing.rs");
    for routing_owner in [
        "fn profiles_for_binding",
        "fn routes_for_binding",
        "fn default_route_for_binding",
        "fn role_intents_for_binding",
        "fn role_intents_for_profiles",
    ] {
        assert!(!hosts.contains(routing_owner));
        assert!(routing.contains(routing_owner));
    }
}
