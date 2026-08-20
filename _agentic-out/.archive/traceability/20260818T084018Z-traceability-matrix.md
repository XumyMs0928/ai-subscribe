---
stepsCompleted: ['step-01-load-context', 'step-02-discover-tests', 'step-03-map-criteria', 'step-04-analyze-gaps', 'step-05-gate-decision']
lastStep: 'step-05-gate-decision'
lastSaved: '2026-08-18'
coverageBasis: 'acceptance_criteria'
oracleConfidence: 'high'
oracleResolutionMode: 'formal_requirements'
oracleSources:
  - '_agentic-out/implementation/stories/2-2-订阅并增量同步-rss-atom-来源.md'
  - '_agentic-out/planning/prd.md'
externalPointerStatus: 'not_used'
tempCoverageMatrixPath: 'C:/Users/13479/AppData/Local/Temp/testing-trace-coverage-matrix-2026-08-18T16-30-00-000Z.json'
---

# Story 2.2 Requirements-to-Tests Traceability Matrix

## Oracle and inventory

- Oracle: the four formal Story 2.2 acceptance criteria; no synthetic or external requirements were used.
- Scope: Windows-first RSS/Atom minimum loop. Mobile platforms and Story 2.5 public task UI remain deferred.
- HTTP API/auth: N/A. The applicable boundaries are outbound core-owned RSS transport and narrow Tauri IPC.
- Relevant evidence inventory: 16 RSS integration tests, 10 production HTTP-policy unit tests, source Tauri/transport tests, 6 source component tests, 2 Story 2.2 E2E tests, plus release-surface/xtask gates.
- Runtime regression result: Rust 126/126, Vitest 81/81, Playwright 23/23.

## Matrix

| ID | Priority | Requirement | Direct evidence | Levels | Coverage |
| --- | --- | --- | --- | --- | --- |
| AC1 | P0 | Only save normalized, publicly routable HTTPS RSS/Atom sources; reject unsafe targets/TLS without bypass | `source_url_policy_is_https_only_and_rejects_non_public_addresses` (`crates/radar-core/tests/rss_atom_sources.rs:104`); `injected_policy_pins_public_resolution_and_sends_conditionals` (`source_http_policy.rs:743`); `rejected_source_write_leaves_configuration_and_sources_unchanged` (`rss_atom_sources.rs:698`); `source_contract_crosses_the_production_command_helper_and_store_recovers` (`commands/mod.rs:482`); source component/E2E save and blocking cases | Unit, Integration, Component, E2E | **FULL** |
| AC2 | P0 | First/subsequent RSS and Atom parsing preserves available provenance and persists validators/cursor without duplicates | `rss_and_atom_fixtures_preserve_optional_fields_and_stable_identity` (`rss_atom_sources.rs:149`); `incremental_application_classifies_new_changed_and_unchanged_candidates` (`:456`); `incremental_harness_reopens_with_validators_cursor_and_persists_retry_after` (`:638`); parser structure/encoding/identity tests (`:310`, `:349`, `:401`) | Unit, Integration | **FULL** |
| AC3 | P0 | Enforce 10 MB, five redirects, 30-second absolute deadline and per-hop SSRF validation while keeping the app operable | raw boundary/compression/deadline tests (`source_http_policy.rs:726`, `:734`, `:940`); private rebinding/public redirect tests (`:772`, `:801`); connector redaction/oversize real-parser tests (`:837`, `:880`, `:915`); UI refresh/storage failure recovery tests | Unit, Integration, Component | **FULL** |
| AC4 | P1 | Source-scoped 429/5xx/parse/connect errors retain retryability and non-shrinking retry timing | authoritative transport cases (`rss_atom_sources.rs:46`); `retry_backoff_is_bounded_by_server_advice_and_never_shrinks` (`:128`); `permanent_failure_and_resave_have_consistent_source_state` (`:557`); injected Retry-After clock (`source_http_policy.rs:855`); component failure-state tests (`sources-page.test.tsx:77`, `:132`) | Unit, Integration, Component | **FULL** |

## Coverage heuristics

| Heuristic | Status | Notes |
| --- | --- | --- |
| HTTP/OpenAPI endpoint gaps | N/A | No inbound HTTP endpoint exists. |
| Tauri IPC boundary gaps | 0 | Exact save/query commands, strict DTO guards, panic recovery and release allowlist are covered. |
| Authentication negative paths | N/A | Story has no account/session/role requirement. |
| Happy-path-only criteria | 0 | All four criteria include direct negative/error-path evidence where applicable. |
| UI journey gaps | 0 | Save/reload and blocking failure/retained input are covered by E2E. |
| UI state gaps | 0 | Loading, empty, ready, refreshing, blocking, storage/config and migration failure states have component coverage. |

## Phase-1 evidence boundary

The release build and static release-surface gates pass. Native GUI smoke and the 30-sample cold-start P95 are intentionally deferred to one run against the frozen phase-1 candidate; this is a release-candidate evidence item, not an AC1–AC4 functional coverage gap.

## Gate decision: PASS

P0 coverage is 3/3 (100%), P1 coverage is 1/1 (100%), and overall acceptance-criteria coverage is 4/4 (100%). No skipped/fixme/pending evidence or open coverage blocker exists. This PASS applies to the Story 2.2 Windows RSS milestone; native GUI and 30-sample performance evidence remain a single deferred phase-1 release-candidate activity.
