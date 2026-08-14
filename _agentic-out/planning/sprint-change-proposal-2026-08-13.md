---
project: ai-subscribe
date: '2026-08-13'
status: applied
approvedAt: '2026-08-13'
appliedAt: '2026-08-13'
mode: incremental
changeScope: moderate
trigger:
  - 'Implementation Readiness Major finding: Story 1.1 exceeds single-agent delivery size'
  - 'Implementation Readiness Major finding: Story 9.5 exceeds single-agent delivery size'
inputDocuments:
  - '_agentic-out/planning/prd.md'
  - '_agentic-out/planning/architecture.md'
  - '_agentic-out/planning/ux-design-specification.md'
  - '_agentic-out/planning/epics.md'
  - '_agentic-out/planning/implementation-readiness-report-2026-08-13.md'
---

# Sprint Change Proposal

## 1. Issue Summary

2026-08-13 Implementation Readiness 检查确认产品定义、UX、Architecture、FR/NFR 追踪和 Story 依赖均完整，但 Story 1.1 与 Story 9.5 超出“一条 Story 可由单个开发代理独立完成”的合理交付尺寸。

- Story 1.1 同时覆盖共享核心、三个平台壳、跨语言绑定、平台副作用、临时密钥租约、依赖锁定与 CI。
- Story 9.5 同时覆盖 Apple 与 Android 的受保护签名/分发环境、隐私披露一致性和五类设备 NFR54 证据汇总。

问题属于 backlog 切片质量缺陷，不是新需求、PRD 误解、架构失败或 MVP 战略变化。

## 2. Impact Analysis

### Epic impact

- Epic 1 的用户目标、FR1–FR3/FR61 覆盖和优先级不变；Story 数从 4 增至 8。
- Epic 9 的用户目标、FR55–FR58 覆盖和优先级不变；Story 数从 5 增至 7。
- Epic 2–8 不变。
- 不新增、删除、合并或重排 Epic。

### Artifact impact

| Artifact | Impact | Required action |
|---|---|---|
| PRD | None | No edit |
| Architecture | None | No edit |
| UX specification / Spine | None | No edit |
| Epics & Stories | Direct | Split and renumber affected Stories; preserve all acceptance ownership |
| Readiness report | Historical evidence | Keep current NEEDS WORK report; rerun after edits |
| Sprint status | Not present | No update applicable |

### Technical and delivery impact

- Shared contracts precede platform shells; platform shells precede cross-platform binding/CI aggregation.
- Apple and Android protected distribution work become independently reviewable.
- Cross-platform release evidence becomes an aggregation Story, not a hidden extension of either platform release Story.
- No rollback, code deletion, scope reduction, new service, new data model, or new user-facing behavior is required.

## 3. Recommended Approach

Choose **Direct Adjustment** within the existing Epic structure.

- Effort: Low for planning edits; implementation work is redistributed rather than expanded.
- Risk: Low, provided every original acceptance criterion is assigned exactly once or intentionally retained as a cross-cutting aggregate check.
- MVP impact: None.
- Schedule implication: No new product scope; delivery checkpoints become smaller and failures become easier to isolate.

Rejected alternatives:

- Rollback: not viable because no implementation has failed and no completed behavior needs reversal.
- MVP review or scope reduction: not viable because the original MVP remains achievable and fully specified.
- New Epic: unnecessary; both issues are internal sizing problems within existing user-value Epics.

## 4. Detailed Change Proposals

### 4.1 Story 1.1 split

#### OLD

`Story 1.1：启动使用共享核心的三端原生应用`

One Story owns Cargo workspace and shared crates, Windows/Tauri, Apple/SwiftUI, Android/Compose, UniFFI round trips, AppError, PlatformEffect outbox, temporary secret leases, dependency locking, and cross-platform CI smoke checks.

#### NEW

1. **Story 1.1：建立共享核心与版本化契约基线**
   - Initialize Cargo workspace, `radar-core`, `radar-ffi`, `contracts`, and `xtask`.
   - Define authoritative DTO, AppError, PlatformEffect, and SecretLeaseInput contracts.
   - Run pure Rust contract tests.
   - Do not create unrelated domain tables up front.

2. **Story 1.2：启动 Windows 原生应用壳并接入共享核心**
   - Initialize Tauri/React Windows shell and DesktopApi boundary.
   - Verify Windows DTO/AppError, effect, and temporary secret-lease round trips.
   - Produce Windows build and contract smoke evidence.

3. **Story 1.3：启动 Apple 原生应用壳并接入共享核心**
   - Initialize SwiftUI Apple shell and RadarCoreClient actor boundary.
   - Verify Apple contract round trips and Keychain-backed temporary secret lease.
   - Produce Apple build and contract smoke evidence.

4. **Story 1.4：启动 Android 原生应用壳并接入共享核心**
   - Initialize Compose Android shell and repository/ViewModel boundary.
   - Verify Android contract round trips and Keystore-backed temporary secret lease.
   - Produce Android build and contract smoke evidence.

5. **Story 1.5：集成生成绑定与跨平台契约质量门禁**
   - Verify generated bindings have no drift across all three platform consumers.
   - Verify PlatformEffect idempotent reporting is consistent.
   - Lock toolchains/dependencies and execute the approved contract and lifecycle CI gates.
   - Aggregate cross-platform contract evidence.

6. Rename original Story 1.2 to Story 1.6.
7. Rename original Story 1.3 to Story 1.7.
8. Rename original Story 1.4 to Story 1.8.

#### Rationale

The split isolates independently failing toolchains while preserving a backward-only sequence: shared contract → Windows → Apple → Android → cross-platform gate → user-facing demonstration and onboarding Stories.

### 4.2 Story 9.5 split

#### OLD

`Story 9.5：通过移动受控测试分发与隐私披露门禁`

One Story owns Apple signing/TestFlight, Android signing/Google Play Internal Testing, seven disclosure categories, declaration-versus-behavior checks, migration/task/privacy evidence, and five-device NFR54 aggregation.

#### NEW

1. **Story 9.5：通过 Apple 受控测试分发门禁**
   - Validate iOS/iPadOS signing, installation eligibility, TestFlight authorization, protected signing-secret boundaries, and Apple disclosure/behavior consistency.

2. **Story 9.6：通过 Android 受控测试分发门禁**
   - Validate signed AAB eligibility, Google Play Internal Testing authorization, debug/PR/fork secret isolation, and Android disclosure/behavior consistency.

3. **Story 9.7：汇总跨平台发布与 GitHub 发现验收证据**
   - Aggregate Windows, Apple, and Android build/runtime evidence.
   - Verify seven disclosure categories are complete and declaration mismatches equal zero.
   - Execute one version of NFR54 fixed fixtures across five device classes.
   - Prohibit live GitHub results as authoritative release evidence.
   - Depend only on completed platform slices and prior capability Stories; do not own signing implementation.

#### Rationale

The split separates protected platform release environments from cross-platform evidence aggregation, allowing credentials, failures, reviewers, and acceptance records to remain independently auditable.

## 5. Coverage and Renumbering Controls

After applying the edits:

- Epic 1 Story sequence must be exactly 1.1–1.8.
- Epic 9 Story sequence must be exactly 9.1–9.7.
- All 64 FRs, 54 NFRs, 44 Architecture requirements, and 37 UX requirements must remain referenced by at least one detailed Story acceptance criterion.
- No Story may depend on a higher-numbered Story in the same Epic.
- Story 1.1 must remain the only initial workspace bootstrap and must not create all future domain tables.
- Story 9.7 must aggregate evidence without gaining signing credentials or duplicating platform release implementation.
- No change may introduce GitHub subscription deletion, private-repository access, GitHub login, silent capacity eviction, or an unlimited-monitoring promise.

## 6. Implementation Handoff

### Scope classification

**Moderate — backlog reorganization.** Product scope and architecture remain stable; Story boundaries and numbering change.

### Recipients and responsibilities

- **Product Owner / planning agent:** apply the approved Story split, preserve acceptance ownership, and update numbering/references.
- **Developer agents:** implement only after the revised Story set passes Readiness; treat each new Story as an independent assignment.
- **Test/release owners:** maintain platform-specific evidence in Stories 9.5/9.6 and aggregate it in Story 9.7.

### Success criteria

1. Revised `epics.md` contains 53 Stories: Epic 1 has 8, Epic 9 has 7, and all other Epic counts remain unchanged.
2. Story numbering is continuous and unique.
3. Requirement coverage remains 100% with zero placeholders.
4. Forward dependency count remains zero.
5. Rerun Readiness returns `READY` with no Major Story-sizing issue.

## 7. Checklist Record

| Checklist section | Status | Finding |
|---|---|---|
| 1. Trigger and context | Done | Two oversized Stories identified by Readiness evidence |
| 2. Epic impact | Done | Epic 1 and 9 internal split only; no Epic-level scope change |
| 3. Artifact conflict | Done | Only `epics.md` requires edits; PRD/Architecture/UX unchanged |
| 4. Path forward | Done | Direct Adjustment selected; rollback and MVP review rejected |
| 5. Proposal components | Done | Issue, impact, approach, edits, handoff, success criteria documented |
| 6. Final review and handoff | Done | User approved; edits applied; all success gates passed |

## 8. Application Record

- User approval: Yes.
- Applied artifact: `_agentic-out/planning/epics.md`.
- Final Story count: 53.
- Epic Story counts: `1:8, 2:6, 3:7, 4:8, 5:4, 6:4, 7:4, 8:5, 9:7`.
- Continuous and unique numbering: Pass.
- FR1–FR64 coverage: Pass.
- NFR1–NFR54 coverage: Pass.
- ARCH-1–ARCH-44 coverage: Pass.
- UX-DR1–UX-DR37 coverage: Pass.
- BDD structure: Pass.
- Forward dependencies: 0.
- Placeholders: 0.
- Sprint-status update: N/A; no sprint-status artifact exists yet.
- Handoff: rerun Implementation Readiness; proceed to Sprint Planning only if the new result is `READY`.
