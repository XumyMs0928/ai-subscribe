use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use radar_core::application::demo::validate_demo_fixture;
use radar_core::application::health_check;
use radar_core::contracts::effects::{EffectLedger, EffectStatus, PlatformEffect, ReportResult};
use radar_core::contracts::manifest::{contract_manifest_json, error_codes_json};
use radar_core::contracts::secrets::SecretLeaseInput;
use radar_ffi::error::map_unknown;

pub fn run_from_args<I>(args: I) -> Result<(), String>
where
    I: IntoIterator<Item = String>,
{
    let args: Vec<_> = args.into_iter().collect();
    if args.as_slice() != ["contracts"] {
        return Err("usage: cargo run -p xtask -- contracts".to_owned());
    }

    let root = workspace_root()?;
    check_equal(
        &root.join("contracts/schemas/contract-manifest-v1.json"),
        &contract_manifest_json(),
    )?;
    check_equal(
        &root.join("contracts/snapshots/error-codes-v1.json"),
        &error_codes_json(),
    )?;
    check_required_fixtures(&root)?;
    check_boundaries(&root)?;
    println!("contracts: PASS (v1 schemas, snapshots, fixtures, boundaries)");
    Ok(())
}

fn workspace_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "unable to resolve workspace root".to_owned())
}

fn check_equal(path: &Path, expected: &str) -> Result<(), String> {
    let actual =
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if compact_json(&actual)? != compact_json(expected)? {
        return Err(format!("contract drift: {}", path.display()));
    }
    Ok(())
}

fn compact_json(value: &str) -> Result<String, String> {
    let mut parser = JsonParser::new(value);
    let mut compact = String::with_capacity(value.len());
    parser.parse_value(&mut compact)?;
    parser.skip_whitespace();
    if parser.position != parser.bytes.len() || !compact.starts_with('{') {
        return Err("invalid JSON: expected one object envelope".to_owned());
    }
    Ok(compact)
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> JsonParser<'a> {
    fn new(value: &'a str) -> Self {
        Self {
            bytes: value.as_bytes(),
            position: 0,
        }
    }

    fn skip_whitespace(&mut self) {
        while self
            .bytes
            .get(self.position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.position += 1;
        }
    }

    fn parse_value(&mut self, output: &mut String) -> Result<(), String> {
        self.skip_whitespace();
        match self.bytes.get(self.position).copied() {
            Some(b'{') => self.parse_collection(output, b'{', b'}', true),
            Some(b'[') => self.parse_collection(output, b'[', b']', false),
            Some(b'"') => self.parse_string(output),
            Some(b't') => self.parse_literal(output, b"true"),
            Some(b'f') => self.parse_literal(output, b"false"),
            Some(b'n') => self.parse_literal(output, b"null"),
            Some(b'-' | b'0'..=b'9') => self.parse_number(output),
            _ => Err("invalid JSON: malformed value".to_owned()),
        }
    }

    fn parse_collection(
        &mut self,
        output: &mut String,
        open: u8,
        close: u8,
        object: bool,
    ) -> Result<(), String> {
        self.position += 1;
        output.push(char::from(open));
        self.skip_whitespace();
        if self.bytes.get(self.position) == Some(&close) {
            self.position += 1;
            output.push(char::from(close));
            return Ok(());
        }
        loop {
            if object {
                self.parse_string(output)?;
                self.skip_whitespace();
                if self.bytes.get(self.position) != Some(&b':') {
                    return Err("invalid JSON: object key is missing a colon".to_owned());
                }
                self.position += 1;
                output.push(':');
            }
            self.parse_value(output)?;
            self.skip_whitespace();
            match self.bytes.get(self.position).copied() {
                Some(value) if value == close => {
                    self.position += 1;
                    output.push(char::from(close));
                    return Ok(());
                }
                Some(b',') => {
                    self.position += 1;
                    output.push(',');
                    self.skip_whitespace();
                }
                _ => return Err("invalid JSON: malformed collection".to_owned()),
            }
        }
    }

    fn parse_string(&mut self, output: &mut String) -> Result<(), String> {
        if self.bytes.get(self.position) != Some(&b'"') {
            return Err("invalid JSON: object key must be a string".to_owned());
        }
        let start = self.position;
        self.position += 1;
        while let Some(byte) = self.bytes.get(self.position).copied() {
            match byte {
                b'"' => {
                    self.position += 1;
                    output.push_str(
                        std::str::from_utf8(&self.bytes[start..self.position])
                            .map_err(|_| "invalid JSON: invalid UTF-8".to_owned())?,
                    );
                    return Ok(());
                }
                b'\\' => {
                    self.position += 1;
                    match self.bytes.get(self.position).copied() {
                        Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => {
                            self.position += 1;
                        }
                        Some(b'u') => {
                            self.position += 1;
                            for _ in 0..4 {
                                if !self
                                    .bytes
                                    .get(self.position)
                                    .is_some_and(u8::is_ascii_hexdigit)
                                {
                                    return Err("invalid JSON: malformed unicode escape".to_owned());
                                }
                                self.position += 1;
                            }
                        }
                        _ => return Err("invalid JSON: malformed escape".to_owned()),
                    }
                }
                0..=0x1f => return Err("invalid JSON: control byte in string".to_owned()),
                _ => self.position += 1,
            }
        }
        Err("invalid JSON: unterminated string".to_owned())
    }

    fn parse_literal(&mut self, output: &mut String, literal: &[u8]) -> Result<(), String> {
        if self.bytes.get(self.position..self.position + literal.len()) != Some(literal) {
            return Err("invalid JSON: malformed literal".to_owned());
        }
        self.position += literal.len();
        output.push_str(std::str::from_utf8(literal).expect("JSON literal is UTF-8"));
        Ok(())
    }

    fn parse_number(&mut self, output: &mut String) -> Result<(), String> {
        let start = self.position;
        if self.bytes.get(self.position) == Some(&b'-') {
            self.position += 1;
        }
        match self.bytes.get(self.position).copied() {
            Some(b'0') => self.position += 1,
            Some(b'1'..=b'9') => {
                self.position += 1;
                while self
                    .bytes
                    .get(self.position)
                    .is_some_and(u8::is_ascii_digit)
                {
                    self.position += 1;
                }
            }
            _ => return Err("invalid JSON: malformed number".to_owned()),
        }
        if self.bytes.get(self.position) == Some(&b'.') {
            self.position += 1;
            let fraction = self.position;
            while self
                .bytes
                .get(self.position)
                .is_some_and(u8::is_ascii_digit)
            {
                self.position += 1;
            }
            if self.position == fraction {
                return Err("invalid JSON: malformed number fraction".to_owned());
            }
        }
        if matches!(self.bytes.get(self.position), Some(b'e' | b'E')) {
            self.position += 1;
            if matches!(self.bytes.get(self.position), Some(b'+' | b'-')) {
                self.position += 1;
            }
            let exponent = self.position;
            while self
                .bytes
                .get(self.position)
                .is_some_and(u8::is_ascii_digit)
            {
                self.position += 1;
            }
            if self.position == exponent {
                return Err("invalid JSON: malformed number exponent".to_owned());
            }
        }
        output.push_str(
            std::str::from_utf8(&self.bytes[start..self.position])
                .map_err(|_| "invalid JSON: invalid UTF-8".to_owned())?,
        );
        Ok(())
    }
}

fn check_required_fixtures(root: &Path) -> Result<(), String> {
    let fixtures = [
        ("health_success_v1.json", HEALTH_FIXTURE),
        ("validation_failure_v1.json", VALIDATION_FIXTURE),
        ("internal_error_v1.json", INTERNAL_FIXTURE),
        ("effect_report_v1.json", EFFECT_FIXTURE),
        ("secret_lease_v1.json", SECRET_FIXTURE),
    ];
    for (name, expected) in fixtures {
        let path = root.join("contracts/fixtures/golden").join(name);
        let contents = fs::read_to_string(&path)
            .map_err(|error| format!("missing fixture {}: {error}", path.display()))?;
        if compact_json(&contents)? != compact_json(expected)? {
            return Err(format!("fixture drift: {}", path.display()));
        }
        for forbidden in ["secret_value", "authorization", "api_key"] {
            if contents.to_ascii_lowercase().contains(forbidden) {
                return Err(format!(
                    "forbidden field `{forbidden}` in {}",
                    path.display()
                ));
            }
        }
    }

    let demo_path = root.join("contracts/fixtures/demo/manifest-v1.json");
    let demo_contents = fs::read_to_string(&demo_path)
        .map_err(|error| format!("missing demo fixture {}: {error}", demo_path.display()))?;
    compact_json(&demo_contents)?;
    validate_demo_fixture(&demo_contents)
        .map_err(|_| format!("demo fixture contract drift: {}", demo_path.display()))?;
    for forbidden in ["secret_value", "authorization", "api_key"] {
        if demo_contents.to_ascii_lowercase().contains(forbidden) {
            return Err(format!(
                "forbidden field `{forbidden}` in {}",
                demo_path.display()
            ));
        }
    }
    execute_fixture_behaviors()?;
    Ok(())
}

fn execute_fixture_behaviors() -> Result<(), String> {
    let health = health_check();
    if health.contract_version != 1 || health.status != "ok" || health.checked_at.is_some() {
        return Err("health_success_v1 behavior mismatch".to_owned());
    }

    let validation =
        PlatformEffect::new_contract_probe("", "probe:fixture", "2026-08-13T00:00:00Z")
            .expect_err("empty effect ID is invalid");
    if validation.code() != "validation.effect_id" {
        return Err("validation_failure_v1 behavior mismatch".to_owned());
    }

    let internal = map_unknown::<()>("private fixture fault")
        .expect_err("unknown fixture error must map to AppError");
    if internal.code() != "internal.unexpected" || internal.category().as_str() != "internal" {
        return Err("internal_error_v1 behavior mismatch".to_owned());
    }

    let effect = PlatformEffect::new_contract_probe(
        "effect:opaque-1",
        "probe:opaque-1",
        "2026-08-13T00:00:00Z",
    )
    .map_err(|error| format!("effect_report_v1 setup failed: {error:?}"))?;
    let mut ledger = EffectLedger::default();
    ledger
        .register(effect)
        .map_err(|error| format!("effect_report_v1 registration failed: {error:?}"))?;
    if ledger.report("effect:opaque-1", "probe:opaque-1", EffectStatus::Delivered)
        != Ok(ReportResult::Applied)
        || ledger.report("effect:opaque-1", "probe:opaque-1", EffectStatus::Delivered)
            != Ok(ReportResult::AlreadyApplied)
    {
        return Err("effect_report_v1 idempotent result mismatch".to_owned());
    }
    let conflict = ledger
        .report("effect:opaque-1", "probe:opaque-1", EffectStatus::Failed)
        .expect_err("different terminal report must conflict");
    if conflict.code() != "conflict.effect_already_reported" {
        return Err("effect_report_v1 conflict mismatch".to_owned());
    }

    let canary = b"runtime-fixture-canary";
    let mut observed = false;
    let mut lease = SecretLeaseInput::new("secret:test", canary.to_vec())
        .map_err(|error| format!("secret_lease_v1 setup failed: {error:?}"))?;
    lease
        .with_secret(|bytes| {
            observed = bytes == canary;
            Ok(())
        })
        .map_err(|error| format!("secret_lease_v1 first use failed: {error:?}"))?;
    if !observed
        || lease
            .with_secret(|_| Ok(()))
            .expect_err("second use must fail")
            .code()
            != "conflict.secret_lease_consumed"
    {
        return Err("secret_lease_v1 behavior mismatch".to_owned());
    }
    Ok(())
}

const HEALTH_FIXTURE: &str = r#"{"scenario":"health_success_v1","input":null,"expected":{"contract_version":1,"status":"ok","checked_at":null},"forbidden_fields":[]}"#;
const VALIDATION_FIXTURE: &str = r#"{"scenario":"validation_failure_v1","input":{"effect_id":""},"expected":{"contract_version":1,"code":"validation.effect_id","category":"validation"},"forbidden_fields":[]}"#;
const INTERNAL_FIXTURE: &str = r#"{"scenario":"internal_error_v1","input":{"fault":"opaque_test_fault"},"expected":{"contract_version":1,"code":"internal.unexpected","category":"internal"},"forbidden_fields":["private_error_text"]}"#;
const EFFECT_FIXTURE: &str = r#"{"scenario":"effect_report_v1","input":{"effect_id":"effect:opaque-1","first":"delivered","repeat":"delivered","conflict":"failed"},"expected":{"first":"applied","repeat":"already_applied","conflict":"conflict.effect_already_reported"},"forbidden_fields":[]}"#;
const SECRET_FIXTURE: &str = r#"{"scenario":"secret_lease_v1","input":{"secret_ref":"secret:test","runtime_provider":"in_memory_fake"},"expected":{"first_use":"available","second_use":"conflict.secret_lease_consumed","observable_canary_hits":0},"forbidden_fields":["secret_bytes","secret_plaintext"]}"#;

fn check_boundaries(root: &Path) -> Result<(), String> {
    check_windows_scope(root)?;
    for path in walk_files(root)? {
        check_file_boundary(root, &path)?;
    }
    Ok(())
}

fn check_file_boundary(root: &Path, path: &Path) -> Result<(), String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    let relative_lower = relative.to_ascii_lowercase();
    let is_windows_source = relative_lower.starts_with("apps/windows/");
    let is_rust_integration_test = extension == "rs" && relative_lower.contains("/tests/");
    let is_frontend_test = matches!(extension.as_str(), "ts" | "tsx")
        && (relative_lower.contains("/test/") || relative_lower.contains(".test."));

    if is_windows_source
        && (relative_lower.contains("/fixtures/")
            || matches!(
                file_name.as_str(),
                "health_success_v1.json"
                    | "validation_failure_v1.json"
                    | "internal_error_v1.json"
                    | "effect_report_v1.json"
                    | "secret_lease_v1.json"
            ))
    {
        return Err(format!(
            "copied contract fixture is not allowed: {}",
            path.display()
        ));
    }
    if matches!(file_name.as_str(), ".env" | "id_rsa" | "id_ed25519")
        || file_name.starts_with(".env.")
    {
        return Err(format!("forbidden sensitive file: {}", path.display()));
    }
    if matches!(
        extension.as_str(),
        "db" | "sqlite" | "sqlite3" | "pem" | "key" | "pfx" | "p12" | "secret"
    ) {
        return Err(format!("forbidden project artifact: {}", path.display()));
    }
    if !is_scannable_extension(&extension) {
        return Ok(());
    }

    let contents =
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let lower = contents.to_ascii_lowercase();
    let normalized_whitespace = lower.split_whitespace().collect::<Vec<_>>().join(" ");
    check_forbidden_content(path, &lower, &normalized_whitespace)?;
    if is_windows_source && !is_rust_integration_test && !is_frontend_test {
        let production_lower = contents.to_ascii_lowercase();
        let production_whitespace = production_lower
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        check_windows_production_content(
            &relative_lower,
            &production_lower,
            &production_whitespace,
        )?;
    }
    Ok(())
}

fn is_scannable_extension(extension: &str) -> bool {
    extension.is_empty()
        || matches!(
            extension,
            "rs" | "toml"
                | "json"
                | "md"
                | "yaml"
                | "yml"
                | "txt"
                | "sql"
                | "cmd"
                | "ps1"
                | "js"
                | "mjs"
                | "ts"
                | "tsx"
                | "html"
                | "css"
        )
}

fn check_forbidden_content(
    path: &Path,
    lower: &str,
    normalized_whitespace: &str,
) -> Result<(), String> {
    let path_lower = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    let forbidden_patterns = [
        (["create", "table"].join(" "), true),
        (["rus", "qlite"].concat(), false),
        (["authorization:", "bearer"].join(" "), false),
        (["client", "secret"].join("_"), false),
        (["private", "key"].join("_"), false),
    ];
    for (forbidden, normalize_whitespace) in forbidden_patterns {
        let approved_demo_database_surface = (normalize_whitespace
            && (path_lower.ends_with("crates/radar-core/src/application/demo.rs")
                || path_lower.contains("/vendor/libsqlite3-sys/")))
            || (!normalize_whitespace
                && forbidden == ["rus", "qlite"].concat()
                && (path_lower.ends_with("crates/radar-core/src/application/demo.rs")
                    || path_lower.ends_with("crates/radar-core/cargo.toml")
                    || path_lower.ends_with("cargo.lock")
                    || path_lower.contains("/vendor/libsqlite3-sys/")));
        if approved_demo_database_surface {
            continue;
        }
        let searchable = if normalize_whitespace {
            normalized_whitespace
        } else {
            lower
        };
        if searchable.contains(&forbidden) {
            return Err(format!(
                "forbidden pattern `{forbidden}` in {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn check_windows_scope(root: &Path) -> Result<(), String> {
    if root.join("migrations").exists() {
        return Err("out-of-scope directory exists: migrations".to_owned());
    }
    for entry in fs::read_dir(root).map_err(|error| format!("{}: {error}", root.display()))? {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name();
        if name.to_string_lossy().eq_ignore_ascii_case("apps") && name != "apps" {
            return Err("out-of-scope platform directory casing: expected `apps`".to_owned());
        }
    }
    let apps = root.join("apps");
    if apps.exists() {
        for entry in fs::read_dir(&apps).map_err(|error| format!("{}: {error}", apps.display()))? {
            let entry = entry.map_err(|error| error.to_string())?;
            if !entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case("windows")
                || entry.file_name() != "windows"
            {
                return Err(format!(
                    "out-of-scope platform directory exists: apps/{}",
                    entry.file_name().to_string_lossy()
                ));
            }
        }
    }
    for forbidden in [
        "apps/windows/src/database",
        "apps/windows/src/stores",
        "apps/windows/src-tauri/migrations",
    ] {
        if root.join(forbidden).exists() {
            return Err(format!("out-of-scope Windows surface exists: {forbidden}"));
        }
    }
    for path in walk_files(root)? {
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/")
            .to_ascii_lowercase();
        if relative.starts_with("apps/windows/") && !is_allowed_windows_shell_path(&relative) {
            return Err(format!("out-of-scope Windows surface exists: {relative}"));
        }
    }
    Ok(())
}

fn is_allowed_windows_shell_path(relative: &str) -> bool {
    const ROOT_FILES: &[&str] = &[
        "apps/windows/components.json",
        "apps/windows/.prettierignore",
        "apps/windows/eslint.config.js",
        "apps/windows/index.html",
        "apps/windows/package.json",
        "apps/windows/tsconfig.app.json",
        "apps/windows/tsconfig.app.tsbuildinfo",
        "apps/windows/tsconfig.json",
        "apps/windows/tsconfig.node.json",
        "apps/windows/tsconfig.node.tsbuildinfo",
        "apps/windows/vite.config.ts",
        "apps/windows/src/main.tsx",
        "apps/windows/src/lib/utils.ts",
        "apps/windows/src/lib/query-client.ts",
        "apps/windows/src-tauri/build.rs",
        "apps/windows/src-tauri/cargo.toml",
        "apps/windows/src-tauri/src/lib.rs",
        "apps/windows/src-tauri/src/main.rs",
        "apps/windows/src-tauri/tauri.conf.json",
    ];
    const PREFIXES: &[&str] = &[
        "apps/windows/src/app/providers/",
        "apps/windows/src/app/shell/",
        "apps/windows/src/features/demo-intelligence/",
        "apps/windows/src/components/ui/",
        "apps/windows/src/lib/desktop-api/",
        "apps/windows/src/styles/",
        "apps/windows/src/test/",
        "apps/windows/src-tauri/capabilities/",
        "apps/windows/src-tauri/icons/",
        "apps/windows/src-tauri/src/commands/",
        "apps/windows/src-tauri/src/platform/",
        "apps/windows/src-tauri/tests/",
    ];
    ROOT_FILES.contains(&relative) || PREFIXES.iter().any(|prefix| relative.starts_with(prefix))
}

fn check_windows_production_content(
    relative: &str,
    lower: &str,
    normalized_whitespace: &str,
) -> Result<(), String> {
    let desktop_transport = "apps/windows/src/lib/desktop-api/tauri-desktop-api.ts";
    let compact = lower
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    if relative.starts_with("apps/windows/src/")
        && relative != desktop_transport
        && (lower.contains("@tauri-apps/api/core")
            || compact.contains("invoke(")
            || compact.contains("__tauri_internals__.invoke(")
            || compact.contains("invoke.call(")
            || compact.contains("__tauri_internals__[\"invoke\"]")
            || compact.contains("__tauri_internals__['invoke']"))
    {
        return Err(format!("raw Tauri invoke outside DesktopApi: {relative}"));
    }
    let imports_test_source = lower.lines().any(|line| {
        let line = line.trim_start();
        line.starts_with("import ") && (line.contains("/test/") || line.contains(".test"))
    });
    if relative.starts_with("apps/windows/src/") && imports_test_source {
        return Err(format!(
            "production import reaches test-only source: {relative}"
        ));
    }

    for forbidden in [
        "@tauri-apps/plugin-shell",
        "@tauri-apps/plugin-fs",
        "node:fs",
        "std::process::command",
        "std::fs::",
        "shell:",
        "fs:",
    ] {
        if lower.contains(forbidden) {
            return Err(format!(
                "forbidden file/shell API `{forbidden}` in {relative}"
            ));
        }
    }

    let without_approved_metadata = lower
        .replace("https://schema.tauri.app/config/2", "")
        .replace("https://ui.shadcn.com/schema.json", "")
        .replace("http://ipc.localhost", "");
    if without_approved_metadata.contains("http://")
        || without_approved_metadata.contains("https://")
    {
        return Err(format!("arbitrary remote origin in {relative}"));
    }

    let logging_call = lower.contains("console.log")
        || compact.contains("println!(")
        || compact.contains("eprintln!(")
        || lower.contains("tracing::");
    let secret_term = ["secret", "canary", "password", "api_key", "authorization"]
        .iter()
        .any(|term| lower.contains(term));
    if logging_call && secret_term {
        return Err(format!("secret logging surface in {relative}"));
    }

    let sql_ddl = ["create", "table"].join(" ");
    if normalized_whitespace.contains(&sql_ddl) {
        return Err(format!("forbidden SQL DDL in {relative}"));
    }
    if relative == "apps/windows/src-tauri/src/lib.rs" {
        let rust_code = rust_without_comments(lower);
        let compact = rust_code
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        let marker = "generate_handler![";
        let handlers = compact.match_indices(marker).collect::<Vec<_>>();
        if handlers.len() != 1 {
            return Err("release command table must contain exactly one handler".to_owned());
        }
        let tail = &compact[handlers[0].0 + marker.len()..];
        let commands = tail
            .split(']')
            .next()
            .ok_or_else(|| "release command handler is malformed".to_owned())?;
        if commands
            != "commands::health_v1,commands::demo_bootstrap_v1,commands::demo_search_v1,commands::demo_list_v1,commands::demo_filter_v1,commands::demo_detail_v1"
        {
            return Err("release command allowlist contains an unapproved command".to_owned());
        }
    }
    Ok(())
}

fn rust_without_comments(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut output = String::with_capacity(source.len());
    let mut position = 0;
    let mut block_depth = 0_u32;
    let mut in_string = false;
    let mut escaped = false;
    while position < bytes.len() {
        if block_depth > 0 {
            if bytes.get(position..position + 2) == Some(b"/*") {
                block_depth += 1;
                position += 2;
            } else if bytes.get(position..position + 2) == Some(b"*/") {
                block_depth -= 1;
                position += 2;
            } else {
                position += 1;
            }
            output.push(' ');
            continue;
        }
        if !in_string && bytes.get(position..position + 2) == Some(b"//") {
            while position < bytes.len() && bytes[position] != b'\n' {
                position += 1;
            }
            output.push('\n');
            continue;
        }
        if !in_string && bytes.get(position..position + 2) == Some(b"/*") {
            block_depth = 1;
            position += 2;
            output.push(' ');
            continue;
        }
        let byte = bytes[position];
        output.push(char::from(byte));
        position += 1;
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
        } else if byte == b'"' {
            in_string = true;
        }
    }
    output
}

fn walk_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    let mut visited = BTreeSet::new();
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("{}: {error}", root.display()))?;
    while let Some(directory) = pending.pop() {
        let canonical = directory
            .canonicalize()
            .map_err(|error| format!("{}: {error}", directory.display()))?;
        if !canonical.starts_with(&canonical_root) || !visited.insert(canonical) {
            return Err(format!(
                "directory link escapes or cycles: {}",
                directory.display()
            ));
        }
        for entry in
            fs::read_dir(&directory).map_err(|error| format!("{}: {error}", directory.display()))?
        {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/")
                .to_ascii_lowercase();
            if matches!(
                relative.as_str(),
                ".toolchains"
                    | ".agents"
                    | ".codex"
                    | ".git"
                    | "node_modules"
                    | "target"
                    | "_agentic-out"
                    | "agentic-workflow"
            ) || relative == "apps/windows/node_modules"
                || relative == "apps/windows/dist"
                || relative == "apps/windows/src-tauri/gen"
            {
                continue;
            }
            let file_type = entry.file_type().map_err(|error| error.to_string())?;
            if file_type.is_symlink() {
                return Err(format!(
                    "directory/file link is not allowed: {}",
                    path.display()
                ));
            }
            if file_type.is_dir() {
                pending.push(path);
            } else {
                files.push(path);
            }
        }
    }
    Ok(files)
}
