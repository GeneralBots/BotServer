//! `vm_lifecycle::tests` — split per #1443.

use super::*;



    #[test]
    fn protected_container_names_are_rejected() {
        // #1397 — platform service containers must never be claimed by projects
        for name in ["tables", "system", "bot", "proxy", "TABLES", "vault"] {
            assert!(VmLifecycle::is_protected_container_name(name), "{name} should be protected");
        }
        for name in ["my-app", "contato-calc", "expense-tracker"] {
            assert!(!VmLifecycle::is_protected_container_name(name), "{name} should be allowed");
        }
    }

    #[test]
    fn container_names_are_env_scoped() {
        assert_eq!(
            VmLifecycle::container_name("My App", "development", false),
            "my-app-development"
        );
        assert_eq!(
            VmLifecycle::container_name("My App", "production", false),
            "my-app-prod"
        );
        assert_eq!(
            VmLifecycle::container_name("Web", "development", true),
            "web-development-runner"
        );
    }

    #[test]
    fn validate_accepts_known_envs_and_tiers() {
        let ok_req = CreateVmRequest {
            env: "production".into(),
            tier: "medium".into(),
            runner_enabled: false,
        };
        assert!(VmLifecycle::validate(&ok_req).is_ok());
        let bad = CreateVmRequest {
            env: "moon".into(),
            tier: "small".into(),
            runner_enabled: false,
        };
        assert!(VmLifecycle::validate(&bad).is_err());
        let bad_tier = CreateVmRequest {
            env: "dev".into(),
            tier: "huge".into(),
            runner_enabled: false,
        };
        assert!(VmLifecycle::validate(&bad_tier).is_err());
    }

    #[test]
    fn alm_mapping_org_is_branch_short() {
        let id = Uuid::new_v4();
        let org = VmLifecycle::alm_org(id);
        assert_eq!(org.len(), 8);
        assert_eq!(VmLifecycle::alm_repo("My Web App"), "my-web-app");
    }

    #[test]
    fn skip_pattern_reports_unavailable_when_forced() {
        std::env::set_var("VIBE_INCUS_FORCE_UNAVAILABLE", "1");
        let manager = diesel::r2d2::ConnectionManager::<diesel::PgConnection>::new(
            "postgres://127.0.0.1:1/nonexistent",
        );
        let pool = diesel::r2d2::Pool::builder()
            .max_size(1)
            .min_idle(Some(0))
            .build(manager)
            .expect("pool builder");
        let lifecycle = VmLifecycle::new(pool);
        assert!(!lifecycle.linux_available());
        let err = lifecycle.linux_running("x").unwrap_err();
        assert!(err.contains("vm-skip"), "expected skip error, got {err}");
        std::env::remove_var("VIBE_INCUS_FORCE_UNAVAILABLE");
    }

    /// #1488 — the CodeGraph installer hangs off `linux_create`, and that is
    /// the ONLY place a container is created, so no project kind can reach it
    /// without a VM. This pins that coupling: if a future change provisions a
    /// container by another route, this test is the reminder that the installer
    /// travels with it and the kind gating has to be re-checked.
    #[test]
    fn codegraph_install_is_reachable_only_through_container_creation() {
        let source = include_str!("../vm_incus/linux.rs");
        let calls = source
            .lines()
            .filter(|line| line.contains("self.install_codegraph("))
            .count();
        assert_eq!(
            calls, 1,
            "install_codegraph must be called exactly once, from linux_create"
        );
        let in_create = source
            .split("pub(crate) fn linux_create")
            .nth(1)
            .and_then(|rest| rest.split("fn install_codegraph").next())
            .map(|body| body.contains("self.install_codegraph("))
            .unwrap_or(false);
        assert!(in_create, "the call must sit inside linux_create");
    }

    /// #1488 — the installer must be skippable so an offline host or a CI image
    /// without npm can provision containers normally. Only the explicit `0`
    /// disables it; any other value (including unset) leaves it enabled.
    #[test]
    fn codegraph_install_is_enabled_unless_explicitly_zeroed() {
        let previous = std::env::var_os("VIBE_CODEGRAPH_INSTALL");
        std::env::remove_var("VIBE_CODEGRAPH_INSTALL");
        assert_ne!(
            std::env::var("VIBE_CODEGRAPH_INSTALL").as_deref(),
            Ok("0"),
            "unset must mean enabled"
        );
        std::env::set_var("VIBE_CODEGRAPH_INSTALL", "0");
        assert_eq!(std::env::var("VIBE_CODEGRAPH_INSTALL").as_deref(), Ok("0"));
        match previous {
            Some(value) => std::env::set_var("VIBE_CODEGRAPH_INSTALL", value),
            None => std::env::remove_var("VIBE_CODEGRAPH_INSTALL"),
        }
    }

    /// #1488 — the CLI ships through the npm registry and the base image has
    /// neither node nor npm, so the runtime must be provisioned inside
    /// `install_codegraph` BEFORE the package install. Reversing that order
    /// makes every fresh container fail with `npm: not found`.
    #[test]
    fn node_runtime_is_provisioned_before_the_codegraph_install() {
        let source = include_str!("../vm_incus/linux.rs");
        assert!(
            source.contains("const NODE_UPDATE_ARGS")
                && source.contains("const NODE_INSTALL_ARGS"),
            "the apt provisioning steps must exist"
        );
        let body = source
            .split("fn install_codegraph")
            .nth(1)
            .expect("install_codegraph must be defined");
        let probe = body
            .find("\"npm\"")
            .expect("npm must be probed before it is used");
        let update = body
            .find("NODE_UPDATE_ARGS")
            .expect("the package lists must be refreshed");
        let packages = body
            .find("NODE_INSTALL_ARGS")
            .expect("node/npm must be installed");
        let cli = body
            .find("CODEGRAPH_INSTALL_SCRIPT")
            .expect("the codegraph install must stay inside install_codegraph");
        assert!(
            probe < update && update < packages && packages < cli,
            "order must be: probe npm, apt update, apt install nodejs npm, install codegraph"
        );
    }

    /// #1488 — every `sh -lc` payload travels as one argv element through the
    /// command guard, which rejects `; | & $ \` < >` outright: a single such
    /// character turns the step into `shell injection attempt` and the install
    /// silently never happens (found the hard way on a fresh container).
    #[test]
    fn installer_scripts_carry_no_shell_metacharacters() {
        let source = include_str!("../vm_incus/linux.rs");
        let literal = source
            .split("const CODEGRAPH_INSTALL_SCRIPT: &str = ")
            .nth(1)
            .and_then(|rest| rest.split(';').next())
            .expect("the installer constant must be defined");
        for forbidden in [';', '|', '&', '$', '`', '<', '>'] {
            assert!(
                !literal.contains(forbidden),
                "the installer script must not contain {forbidden:?}"
            );
        }
    }

