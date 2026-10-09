#[cfg(test)]
mod harness_cmd_tests {
    use super::*;

    #[test]
    pub(super) fn runs_node_expression() {
        // Self-contained: write the fixture into a temp dir instead of
        // depending on a pre-seeded /tmp/vibe-workspaces/calculator tree.
        let dir = std::env::temp_dir().join(format!("vibe-cmd-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("index.js"), "console.log(eval(process.argv[2]));").unwrap();
        let out = run(
            "node",
            &["index.js".to_string(), "2+3".to_string()],
            &dir,
            10,
        );
        let _ = std::fs::remove_dir_all(&dir);
        match out {
            Ok(o) => {
                assert_eq!(o.exit_code, Some(0));
                assert_eq!(o.stdout.trim(), "5");
            }
            Err(e) => panic!("node run failed: {e:?}"),
        }
    }
}
