# Story 4.5 Edge Case Hunter

先完整读取并遵循 `D:/2026/TEST1/.agents/skills/agentic-edge-case-review/SKILL.md`。

允许只读项目源码以理解调用链，但不得修改文件或运行测试。审查范围与 `blind-hunter.md` 的 Group 1/2/3 文件一致。重点覆盖：空/极限输入、Unicode、时间边界、cursor 篡改与跨查询复用、配置漂移、并发/迟到响应、SQLite snapshot/排序/分页、transport exactness、直接深链、刷新/下一页失败、选择/焦点/滚动恢复、缓存/离线状态、性能证据同源性与失败清理。

每项 finding 必须包含精确文件/行号、可复现触发条件、用户或数据影响、最小修复；只报告未处理且真实可证的问题。按 Group 1/2/3 分节；无发现明确 `None`。
