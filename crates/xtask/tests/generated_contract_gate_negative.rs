mod contracts_under_test {
    include!("../src/contracts.rs");

    #[cfg(test)]
    mod generated_tests {
        use super::{check_boundaries, check_equal, compact_json, run_from_args};
        use radar_core::application::demo::validate_demo_fixture;
        use std::fs;
        use std::io;
        use std::path::{Path, PathBuf};
        use std::sync::atomic::{AtomicU64, Ordering};

        static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

        #[test]
        fn demo_fixture_rejects_structural_and_semantic_mutations() {
            let fixture = include_str!("../../../contracts/fixtures/demo/manifest-v1.json");
            for invalid in [
                fixture.replacen("\"dataset_id\"", "\"bait\":{},\"dataset_id\"", 1),
                fixture.replacen("\"data_origin\": \"demo\"", "\"data_origin\": \"real\"", 1),
                fixture.replacen("https://openai.com/", "https:///", 1),
                fixture.replacen("2026-07-01T08:00:00Z", "Z", 1),
            ] {
                assert!(validate_demo_fixture(&invalid).is_err());
            }
        }

        struct TempDir {
            path: PathBuf,
        }

        impl TempDir {
            fn new(label: &str) -> Self {
                let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
                let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../target/xtask-test-sandboxes")
                    .join(format!("{label}-{}-{sequence}", std::process::id()));
                fs::create_dir_all(path.parent().expect("sandbox parent"))
                    .expect("test sandbox parent must be created");
                fs::create_dir(&path).expect("unique std-only temp directory must be created");
                Self { path }
            }

            fn path(&self) -> &Path {
                &self.path
            }
        }

        impl Drop for TempDir {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.path);
            }
        }

        fn write_file(root: &Path, relative: &str, contents: &str) -> PathBuf {
            let path = root.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("test parent directory must be created");
            }
            fs::write(&path, contents).expect("test file must be written");
            path
        }

        #[cfg(windows)]
        fn create_file_symlink(target: &Path, link: &Path) -> io::Result<()> {
            std::os::windows::fs::symlink_file(target, link)
        }

        #[cfg(unix)]
        fn create_file_symlink(target: &Path, link: &Path) -> io::Result<()> {
            std::os::unix::fs::symlink(target, link)
        }

        /// [P0] Only the exact contracts command is accepted by the xtask boundary.
        #[test]
        fn xtask_rejects_invalid_cli_argument_shapes() {
            let usage = "usage: cargo run -p xtask -- contracts";
            for args in [
                Vec::<String>::new(),
                vec!["boundaries".to_owned()],
                vec!["contracts".to_owned(), "extra".to_owned()],
            ] {
                assert_eq!(run_from_args(args), Err(usage.to_owned()));
            }
        }

        /// [P0] A valid but changed JSON artifact is reported as contract drift.
        #[test]
        fn xtask_detects_semantic_contract_drift() {
            let temp = TempDir::new("drift");
            let path = write_file(
                temp.path(),
                "contract.json",
                "{\n  \"contract_version\": 2,\n  \"status\": \"ok\"\n}",
            );

            let error = check_equal(&path, "{\"contract_version\":1,\"status\":\"ok\"}")
                .expect_err("changed JSON must fail the drift gate");

            assert!(error.starts_with("contract drift:"));
            assert!(error.contains("contract.json"));
        }

        /// [P0] Broken envelopes, unterminated strings, and dangling escapes are rejected before comparison.
        #[test]
        fn xtask_rejects_invalid_json_envelopes() {
            let invalid_json = [
                "",
                "[]",
                "{\"field\":\"unterminated}",
                "{\"field\":\"dangling\\",
                "not-json",
            ];

            for value in invalid_json {
                let error = compact_json(value).expect_err("invalid JSON must be rejected");
                assert!(error.starts_with("invalid JSON:"), "{value:?}: {error}");
            }
            assert!(compact_json(r#"{"input":n u l l}"#).is_err());
        }

        /// [P1] Sensitive names, artifact extensions, and high-privilege content patterns fail the workspace boundary.
        #[test]
        fn xtask_rejects_sensitive_files_and_contents() {
            let cases = [
                (".env", "safe=value".to_owned(), "forbidden sensitive file"),
                (
                    ".env.local",
                    "safe=value".to_owned(),
                    "forbidden sensitive file",
                ),
                ("id_rsa", "opaque".to_owned(), "forbidden sensitive file"),
                (
                    "id_ed25519",
                    "opaque".to_owned(),
                    "forbidden sensitive file",
                ),
                (
                    "identity.pem",
                    "opaque".to_owned(),
                    "forbidden project artifact",
                ),
                (
                    "cache.sqlite3",
                    "opaque".to_owned(),
                    "forbidden project artifact",
                ),
                (
                    "cache.db",
                    "opaque".to_owned(),
                    "forbidden project artifact",
                ),
                (
                    "cache.sqlite",
                    "opaque".to_owned(),
                    "forbidden project artifact",
                ),
                (
                    "identity.key",
                    "opaque".to_owned(),
                    "forbidden project artifact",
                ),
                (
                    "identity.pfx",
                    "opaque".to_owned(),
                    "forbidden project artifact",
                ),
                (
                    "identity.p12",
                    "opaque".to_owned(),
                    "forbidden project artifact",
                ),
                (
                    "runtime.secret",
                    "opaque".to_owned(),
                    "forbidden project artifact",
                ),
                (
                    "settings.toml",
                    ["authorization:", "bearer opaque-token"].join(" "),
                    "forbidden pattern",
                ),
                (
                    "config.yaml",
                    ["client", "secret: opaque"].join("_"),
                    "forbidden pattern",
                ),
                (
                    "config.txt",
                    ["private", "key: opaque"].join("_"),
                    "forbidden pattern",
                ),
            ];

            for (relative, contents, expected) in cases {
                let temp = TempDir::new("sensitive");
                write_file(temp.path(), relative, &contents);
                let error =
                    check_boundaries(temp.path()).expect_err("sensitive artifact must fail");
                assert!(
                    error.contains(expected),
                    "{relative} returned unexpected error: {error}"
                );
            }
        }

        /// [P1] Every forbidden future-scope surface has an independent mutation test.
        #[test]
        fn xtask_rejects_each_out_of_scope_workspace_surface() {
            let directory_cases = [
                "apps/apple/placeholder.txt",
                "apps/android/placeholder.txt",
                "migrations/001.sql",
            ];
            for relative in directory_cases {
                let temp = TempDir::new("scope-directory");
                write_file(temp.path(), relative, "placeholder");
                let error = check_boundaries(temp.path())
                    .expect_err("out-of-scope directory must fail the boundary gate");
                assert!(error.contains("out-of-scope"), "{error}");
            }

            let content_cases = [
                (
                    "schema.sql",
                    ["create", "table future_items (id integer)"].join(" "),
                    ["create", "table"].join(" "),
                ),
                (
                    "schema-newline.sql",
                    ["create", "table future_items (id integer)"].join("\n"),
                    ["create", "table"].join(" "),
                ),
                (
                    "schema-tab.sql",
                    ["create", "table future_items (id integer)"].join("\t"),
                    ["create", "table"].join(" "),
                ),
                (
                    "schema-spaces.sql",
                    ["create", "table future_items (id integer)"].join("   "),
                    ["create", "table"].join(" "),
                ),
                (
                    "schema",
                    ["create", "table future_items (id integer)"].join("\n"),
                    ["create", "table"].join(" "),
                ),
                (
                    "Cargo.toml",
                    ["[dependencies]", &["rus", "qlite = \"0.1\""].concat()].join("\n"),
                    ["rus", "qlite"].concat(),
                ),
            ];
            for (relative, contents, expected_pattern) in content_cases {
                let temp = TempDir::new("scope-content");
                write_file(temp.path(), relative, &contents);
                let error = check_boundaries(temp.path())
                    .expect_err("out-of-scope content must fail the boundary gate");
                assert!(error.contains(&expected_pattern), "{error}");
            }
        }

        /// [P0] The approved Story 1.6 Windows shell remains allowed.
        #[test]
        fn xtask_allows_the_minimal_windows_shell() {
            let temp = TempDir::new("windows-allowlist");
            write_file(
                temp.path(),
                "apps/windows/src/lib/desktop-api/tauri-desktop-api.ts",
                "import { invoke } from \"@tauri-apps/api/core\"; export const health = () => invoke(\"health_v1\");",
            );
            write_file(
                temp.path(),
                "apps/windows/src-tauri/src/lib.rs",
                concat!(
                    "fn register() { tauri::generate_handler![",
                    "commands::health_v1,",
                    "commands::demo_bootstrap_v1,",
                    "commands::demo_search_v1,",
                    "commands::demo_list_v1,",
                    "commands::demo_filter_v1,",
                    "commands::demo_detail_v1",
                    "]; }"
                ),
            );

            check_boundaries(temp.path()).expect("approved Windows shell must pass");
        }

        /// [P0] Each Windows-only attack surface independently fails the evolved boundary gate.
        #[test]
        fn xtask_rejects_windows_boundary_mutations() {
            let cases = [
                (
                    "apps/windows/src/app/shell/view.tsx",
                    "import { invoke } from \"@tauri-apps/api/core\"; invoke(\"health_v1\");",
                    "raw Tauri invoke",
                ),
                (
                    "apps/windows/src/app/shell/remote.ts",
                    "fetch(\"https://example.invalid/data\");",
                    "arbitrary remote origin",
                ),
                (
                    "apps/windows/src-tauri/src/platform/windows/unsafe_file.rs",
                    "use std::fs; fn open() { let _ = std::fs::read(\"x\"); }",
                    "forbidden file/shell API",
                ),
                (
                    "apps/windows/src/app/shell/plugin.ts",
                    "import \"@tauri-apps/plugin-shell\";",
                    "forbidden file/shell API",
                ),
                (
                    "apps/windows/src/app/shell/diagnostic.ts",
                    "const secret = \"opaque\"; console.log(secret);",
                    "secret logging surface",
                ),
                (
                    "apps/windows/src-tauri/src/lib.rs",
                    "fn register() { tauri::generate_handler![commands::execute]; }",
                    "unapproved command",
                ),
                (
                    "apps/windows/src-tauri/src/lib.rs",
                    "// tauri::generate_handler![commands::health_v1]\nfn register() { tauri::generate_handler![commands::execute]; }",
                    "unapproved command",
                ),
                (
                    "apps/windows/src/app/shell/hidden.ts",
                    "// #[cfg(test)]\nwindow.__TAURI_INTERNALS__.invoke (\"health_v1\");",
                    "raw Tauri invoke",
                ),
                (
                    "apps/windows/src/app/shell/indirect.ts",
                    "window.__TAURI_INTERNALS__[\"invoke\"].call(null, \"health_v1\");",
                    "raw Tauri invoke",
                ),
                (
                    "apps/windows/src/gen/hidden.ts",
                    "export const hidden = true;",
                    "out-of-scope Windows surface",
                ),
                (
                    "apps/windows/src/database/schema.sql",
                    concat!("create", " table future_items (id integer)"),
                    "out-of-scope Windows surface",
                ),
                (
                    "apps/windows/src/app/shell/cache.sqlite",
                    "opaque",
                    "forbidden project artifact",
                ),
                (
                    "apps/windows/src/test/fixtures/health_success_v1.json",
                    "{}",
                    "copied contract fixture",
                ),
                (
                    "apps/windows/src/Fixtures/demo.json",
                    "{}",
                    "out-of-scope Windows surface",
                ),
            ];

            for (relative, contents, expected) in cases {
                let temp = TempDir::new("windows-mutation");
                write_file(temp.path(), relative, contents);
                let error =
                    check_boundaries(temp.path()).expect_err("Windows boundary mutation must fail");
                assert!(error.contains(expected), "{relative}: {error}");
            }

            let temp = TempDir::new("bundled-test-source");
            write_file(
                temp.path(),
                "apps/windows/src/test/bridge.test.ts",
                "export const bridge = () => window.__TAURI_INTERNALS__.invoke (\"health_v1\");",
            );
            write_file(
                temp.path(),
                "apps/windows/src/main.tsx",
                "import './test/bridge.test';",
            );
            let error = check_boundaries(temp.path())
                .expect_err("production must not import an exempt test module");
            assert!(
                error.contains("production import reaches test-only source"),
                "{error}"
            );
        }

        /// [P1] Ignored build and workflow directories are pruned before sensitive-content scanning.
        #[test]
        fn xtask_prunes_ignored_directories_before_scanning() {
            let temp = TempDir::new("ignored");
            let ignored_sensitive_content = ["authorization:", "bearer ignored"].join(" ");
            for relative in [
                "target/nested/.env",
                "_agentic-out/private.pem",
                ".toolchains/id_rsa",
                ".agents/cache.sqlite",
            ] {
                write_file(temp.path(), relative, &ignored_sensitive_content);
            }
            write_file(temp.path(), "src/safe.rs", "pub const VALUE: u32 = 1;");

            check_boundaries(temp.path()).expect("ignored directories must be pruned");
        }

        /// [P1] File links are refused so boundary scans cannot follow aliases or escape the selected root.
        #[test]
        fn xtask_rejects_symbolic_links() {
            let temp = TempDir::new("symlink");
            let root = temp.path().join("workspace");
            fs::create_dir(&root).expect("workspace directory must be created");
            let target = write_file(temp.path(), "outside.txt", "opaque");
            let link = root.join("linked.txt");

            match create_file_symlink(&target, &link) {
                Ok(()) => {}
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::PermissionDenied | io::ErrorKind::Unsupported
                    ) || error.raw_os_error() == Some(1314) =>
                {
                    eprintln!("symlink assertion skipped because this host forbids link creation");
                    return;
                }
                Err(error) => panic!("unable to create test symlink: {error}"),
            }

            let error = check_boundaries(&root).expect_err("symbolic link must fail");
            assert!(error.contains("directory/file link is not allowed"));
            assert!(error.contains("linked.txt"));
        }
    }
}
