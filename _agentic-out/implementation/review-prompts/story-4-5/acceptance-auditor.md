# Story 4.5 Acceptance Auditor

权威规格：`D:/2026/TEST1/_agentic-out/implementation/stories/4-5-浏览高价值主情报流并组合筛选.md`。

完整读取规格，然后只读审查 `blind-hunter.md` 所列 Group 1/2/3 实现文件；不得修改文件或运行测试。逐项核对 AC1–AC7、Tasks 1–7、Phase 1 Windows RSS scope、Deferred 禁止项和 performance evidence。

特别检查：

- high-value 与 ordinary-candidate 是否都真实可达且只消费 core current projection；
- 四维筛选、同维 OR/跨维 AND、effective time、稳定排序与 cursor identity 是否完整；
- demo/real、facts/rules/AI、browser mock/core evidence 是否严格分层；
- loading/refresh/empty/partial/blocking/pagination error、selection/focus/scroll 是否满足规格；
- 50k × 30 × 2、P95、query plan、dataset/candidate/source hash 是否为当前实现同源证据；
- 是否偷跑或伪造处理状态、收藏、搜索、详情/原文、AI、通知、其他来源、移动端或完整发布矩阵。

每项 finding 输出：严重度、标题、违反的 AC/约束、精确文件/行号证据、触发条件和修复建议。按 Group 1/2/3 分节；无发现明确 `None`。
