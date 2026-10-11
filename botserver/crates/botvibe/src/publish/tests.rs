//! `publish::tests` — split per #1443.

use super::*;



    fn test_project(name: &str) -> Project {
        let now = chrono::Utc::now();
        Project {
            id: Uuid::new_v4(),
            org_id: Uuid::nil(),
            branch_id: Uuid::nil(),
            name: name.to_string(),
            project_type: "app-htmx".to_string(),
            repository: String::new(),
            framework: None,
            custom_domain: None,
            source_control: "native".to_string(),
            status: "active".to_string(),
            environment: "development".to_string(),
            payload: Value::Null,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn collect_workspace_files_packages_source_and_skips_vcs() {
        let _guard = harness::WORKSPACE_ENV_LOCK
            .lock()
            .expect("workspace env lock");
        let previous = std::env::var_os("VIBE_WORKSPACE_ROOT");
        let tmp = std::env::temp_dir().join(format!("vibe-publish-test-{}", Uuid::new_v4()));
        std::env::set_var("VIBE_WORKSPACE_ROOT", &tmp);
        let project = test_project("My Web App");
        let slug = VmLifecycle::alm_repo(&project.name);
        let dir = harness::workspace_root().join(&slug);
        std::fs::create_dir_all(dir.join("src")).expect("mkdir src");
        std::fs::create_dir_all(dir.join(".git")).expect("mkdir .git");
        std::fs::write(dir.join("README.md"), b"# demo").expect("write README");
        std::fs::write(dir.join("src/main.rs"), b"fn main() {}").expect("write main");
        std::fs::write(dir.join(".git/config"), b"[core]").expect("write git config");

        let files = collect_workspace_files(&project).expect("collect files");
        let paths: Vec<String> = files
            .iter()
            .map(|f| f["path"].as_str().unwrap_or_default().to_string())
            .collect();
        assert!(paths.contains(&"README.md".to_string()), "paths: {paths:?}");
        assert!(
            paths.contains(&"src/main.rs".to_string()),
            "paths: {paths:?}"
        );
        assert!(
            !paths.iter().any(|p| p.starts_with(".git")),
            "VCS dirs must be excluded: {paths:?}"
        );
        let main = files
            .iter()
            .find(|f| f["path"] == "src/main.rs")
            .expect("main.rs");
        let content: Vec<u8> =
            serde_json::from_value(main["content"].clone()).expect("content bytes");
        assert_eq!(content, b"fn main() {}");

        let _ = std::fs::remove_dir_all(&tmp);
        harness::restore_workspace_root(previous);
    }

