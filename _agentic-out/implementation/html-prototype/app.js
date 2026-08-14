"use strict";

const intelligence = [
  {
    id: "sig-001", title: "OpenWeights 发布端侧推理工具链更新", source: "OpenWeights Blog", time: "今天 09:18",
    summary: "新增量化导出与移动端运行时检查，可能缩短本地 Agent 的验证周期。", track: "tooling", trackLabel: "开发工具", importance: "critical", importanceLabel: "重大动态",
    confidence: "中高（78%）", what: "工具链加入 4-bit 量化导出、算子兼容性报告和设备内性能快照。",
    why: "它直接影响当前端侧 Agent 的打包和性能诊断路径，但真实设备收益仍需核验。",
    impact: "可减少手工转换步骤；旧模型转换配置可能需要迁移。", fact: "发布说明列出三个新命令和两个弃用参数。",
    rule: "命中：端侧推理 + Agent；来源可信度：官方；重大动态阈值：72。", publisher: "OpenWeights", author: "工程团队", originalTitle: "Mobile Runtime Toolchain 2.4", urlLabel: "模拟来源（不打开网络）", availability: "官方来源 · 元数据可用", related: "sig-002"
  },
  {
    id: "sig-002", title: "主流模型网关调整批处理速率限制", source: "Vendor Status", time: "今天 08:36",
    summary: "批处理窗口缩短，并发上限按账户层级变化；现有任务可能需要退避策略。", track: "model", trackLabel: "基础模型", importance: "high", importanceLabel: "高价值",
    confidence: "高（86%）", what: "供应商更新批处理请求的并发和重试窗口。", why: "高峰期任务可能更早进入限流，需核对本地队列策略。",
    impact: "同步延迟可能增加，但不会影响已缓存的情报。", fact: "状态页记录了窗口参数变更，未公布所有账户层级数值。",
    rule: "命中：速率限制；来源可信度：官方状态页；重要度阈值：72。", publisher: "Vendor Status", author: "未提供", originalTitle: "Batch API policy update", urlLabel: "模拟来源（不打开网络）", availability: "官方来源 · 当前限流演示", related: "sig-001"
  },
  {
    id: "sig-003", title: "应用商店补充生成式 AI 数据披露要求", source: "Developer Policy", time: "昨天 18:10",
    summary: "审核表单新增数据发送范围与保留期限说明，影响移动端发布材料。", track: "policy", trackLabel: "政策合规", importance: "critical", importanceLabel: "重大动态",
    confidence: "高（91%）", what: "开发者提交应用时需更明确说明生成式 AI 数据处理边界。", why: "若产品使用第三方模型，发布材料与应用内授权说明需要一致。",
    impact: "可能增加一次合规核对，不影响已发布版本的本地数据。", fact: "政策页新增数据类别、保留期限和第三方处理方字段。",
    rule: "命中：AI 数据 + 政策；来源可信度：官方；重大动态阈值：72。", publisher: "Developer Policy", author: "政策团队", originalTitle: "Generative AI disclosure guidance", urlLabel: "模拟来源（不打开网络）", availability: "官方来源 · 元数据可用", related: "sig-001"
  },
  {
    id: "sig-004", title: "社区提出新的 Agent 工具调用评测集", source: "Research Forum", time: "昨天 15:22",
    summary: "覆盖失败恢复与多步工具选择，数据集仍处预览阶段。", track: "model", trackLabel: "基础模型", importance: "normal", importanceLabel: "普通候选",
    confidence: "待分析", what: "研究社区公开一个预览版工具调用评测集。", why: "可能补充故障恢复场景，但尚未经过独立复现。",
    impact: "暂不建议调整正式评测门槛，可加入待研究。", fact: "当前仅有方法说明和少量样例，完整许可未提供。",
    rule: "命中：Agent + 评测；社区来源；普通候选。", publisher: "Research Forum", author: "未提供", originalTitle: "Agent Tool Recovery Benchmark Preview", urlLabel: "模拟来源（不打开网络）", availability: "社区来源 · 等待分析", related: "sig-002", aiWaiting: true
  }
];

const state = {
  view: "feed", query: "", importance: "all", track: "all", selectedId: intelligence[0].id,
  bookmarked: new Set(), research: new Set(), feedback: new Map(), scenario: "partial",
  savedRules: { keywords: "推理, agent, 开源模型", exclusions: "招聘, 融资传闻", threshold: 72 },
  lastUndo: null, modalTrigger: null, modalItemId: null, feedbackDrafts: new Map(), retryCount: 0, rulesPending: false
};

const $ = (id) => document.getElementById(id);
const els = {
  feedView: $("view-feed"), feedList: $("feed-list"), detail: $("detail-content"), detailPane: $("detail-pane"),
  search: $("search-input"), importance: $("importance-filter"), track: $("track-filter"), filters: $("active-filters"),
  summary: $("feed-results-summary"), empty: $("empty-state"), emptyDescription: $("empty-description"), banner: $("health-banner"),
  modalBackdrop: $("modal-backdrop"), modal: $("modal"), modalTitle: $("modal-title"), modalBody: $("modal-body"), modalActions: $("modal-actions"),
  snackbar: $("snackbar"), snackbarMessage: $("snackbar-message"), snackbarAction: $("snackbar-action")
};

function escapeHtml(value) {
  return String(value).replace(/[&<>"]/g, (char) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[char]);
}

function filteredItems() {
  const query = state.query.trim().toLocaleLowerCase("zh-CN");
  return intelligence.filter((item) => {
    const text = `${item.title} ${item.summary} ${item.source}`.toLocaleLowerCase("zh-CN");
    return (!query || text.includes(query)) && (state.importance === "all" || item.importance === state.importance) && (state.track === "all" || item.track === state.track);
  });
}

function renderFeed() {
  const items = filteredItems();
  const tabbableId = items.some((item) => item.id === state.selectedId) ? state.selectedId : items[0]?.id;
  els.feedList.innerHTML = items.map((item) => `
    <button class="feed-item" type="button" role="option" data-id="${item.id}" tabindex="${item.id === tabbableId ? "0" : "-1"}" aria-selected="${item.id === state.selectedId}" aria-label="${escapeHtml(item.title)}，${item.importanceLabel}${state.bookmarked.has(item.id) ? "，已收藏" : ""}">
      <span class="feed-item-meta"><span>${escapeHtml(item.source)} · ${item.time}</span><span>${state.bookmarked.has(item.id) ? "★ 已收藏" : ""}</span></span>
      <span class="feed-item-title">${escapeHtml(item.title)}</span>
      <span class="feed-item-summary">${escapeHtml(item.summary)}</span>
      <span class="badges"><span class="badge ${item.importance}">${item.importance === "critical" ? "! " : ""}${item.importanceLabel}</span><span class="badge">${item.trackLabel}</span>${item.aiWaiting ? '<span class="badge ai">AI · 等待分析</span>' : '<span class="badge ai">AI 生成摘要</span>'}</span>
    </button>`).join("");
  els.summary.textContent = `显示 ${items.length} 条演示情报；搜索范围：元数据、必要摘录与来源。`;
  els.empty.hidden = items.length > 0;
  els.feedList.hidden = items.length === 0;
  if (!items.length) {
    const conditions = [state.query && `查询“${state.query}”`, state.importance !== "all" && els.importance.selectedOptions[0].text, state.track !== "all" && els.track.selectedOptions[0].text].filter(Boolean).join("、");
    els.emptyDescription.textContent = `${conditions || "当前条件"}没有匹配项。缓存仍安全可用，可清除条件继续浏览。`;
  }
  renderFilterChips();
}

function renderFilterChips() {
  const chips = [];
  if (state.query) chips.push(["query", `搜索：${state.query}`]);
  if (state.importance !== "all") chips.push(["importance", `重要度：${els.importance.selectedOptions[0].text}`]);
  if (state.track !== "all") chips.push(["track", `赛道：${els.track.selectedOptions[0].text}`]);
  els.filters.innerHTML = chips.map(([key, label]) => `<button class="chip" type="button" data-clear="${key}" aria-label="移除条件 ${escapeHtml(label)}">${escapeHtml(label)} ×</button>`).join("") + (chips.length ? '<button class="chip" type="button" data-clear="all">恢复默认</button>' : "");
}

function selectedItem() { return intelligence.find((item) => item.id === state.selectedId); }

function renderDetail() {
  const item = selectedItem();
  if (!item) {
    els.detail.innerHTML = '<div class="empty-state"><h1 id="detail-title">目标不可用</h1><p>该模拟深链目标不存在，未触发同步、分析或通知。请安全返回主情报流。</p><button class="button primary" type="button" data-action="safe-return">返回主情报流</button></div>';
    return;
  }
  const bookmarkLabel = state.bookmarked.has(item.id) ? "已收藏" : "收藏";
  const researchLabel = state.research.has(item.id) ? "已加入待研究" : "加入待研究";
  const feedback = state.feedback.get(item.id);
  els.detail.innerHTML = `
    <header class="detail-header">
      <div class="badges"><span class="badge ${item.importance}">${item.importance === "critical" ? "! " : ""}${item.importanceLabel}</span><span class="badge">${item.trackLabel}</span><span class="badge ai">${item.aiWaiting ? "AI · 等待分析" : "AI 生成摘要"}</span></div>
      <h1 id="detail-title">${escapeHtml(item.title)}</h1>
      <p class="meta">${escapeHtml(item.source)} · ${item.time} · 演示数据</p>
      <div class="detail-actions" aria-label="情报操作">
        <button class="button primary" type="button" data-action="original">查看模拟原文</button>
        <button class="button" type="button" data-action="bookmark" aria-pressed="${state.bookmarked.has(item.id)}">${bookmarkLabel}</button>
        <button class="button" type="button" data-action="research" aria-pressed="${state.research.has(item.id)}">${researchLabel}</button>
        <button class="button" type="button" data-action="feedback">${feedback ? `反馈：${feedback}` : "提供反馈"}</button>
      </div>
    </header>
    <section class="detail-section" aria-labelledby="what-heading"><p class="semantic-label">原始事实与必要摘录</p><h2 id="what-heading">发生了什么</h2><p>${escapeHtml(item.what)}</p></section>
    <section class="detail-section" aria-labelledby="why-heading"><p class="semantic-label">AI 生成 · 请核验来源</p><h2 id="why-heading">为什么重要</h2><p>${escapeHtml(item.why)}</p><h3>可能影响</h3><p>${escapeHtml(item.impact)}</p></section>
    <section class="detail-section facts" aria-label="判断摘要">
      <div class="fact"><span class="meta">重要程度</span><strong>${item.importanceLabel}</strong></div>
      <div class="fact"><span class="meta">AI 置信度</span><strong>${item.confidence}</strong></div>
      <div class="fact"><span class="meta">来源可信度</span><strong>${item.availability.includes("官方") ? "官方来源" : "待交叉核验"}</strong></div>
    </section>
    <details id="evidence-details"><summary>查看评分依据、原始事实与完整溯源</summary><div class="evidence-body">
      <h2>规则依据</h2><p>${escapeHtml(item.rule)}</p>
      <h2>原始事实</h2><p>${escapeHtml(item.fact)}</p>
      <h2>来源溯源</h2><dl class="provenance"><dt>发布方</dt><dd>${escapeHtml(item.publisher)}</dd><dt>作者</dt><dd>${escapeHtml(item.author)}</dd><dt>原始标题</dt><dd>${escapeHtml(item.originalTitle)}</dd><dt>链接</dt><dd>${item.urlLabel}</dd><dt>采集时间</dt><dd>${item.time}</dd><dt>可用状态</dt><dd>${escapeHtml(item.availability)}</dd><dt>关联依据</dt><dd>${escapeHtml(item.related)}（演示标识）</dd></dl>
    </div></details>`;
}

function renderHealthBanner() {
  const content = {
    healthy: ["success", "全部模拟来源已完成 · 缓存与最新结果可用 · 未产生真实副作用"],
    offline: ["warning", "离线可用 · 展示设备缓存 · 外部动作暂停 · 数据未丢失"],
    partial: ["warning", "部分成功 · 3 个来源完成，Vendor RSS 受限 · 已成功结果和缓存保持可用"],
    rate: ["warning", "Vendor RSS 同步受限 · 失败阶段：请求 · 影响 1 个来源 · 可在诊断中查看恢复条件"],
    ai: ["warning", "AI 等待 · 原始标题、来源和摘录仍可用 · 不生成兜底结论"]
  }[state.scenario];
  els.banner.dataset.tone = content[0];
  els.banner.textContent = content[1];
}

function updateFilters() {
  state.query = els.search.value;
  state.importance = els.importance.value;
  state.track = els.track.value;
  renderFeed();
}

function resetFilters() {
  els.search.value = ""; els.importance.value = "all"; els.track.value = "all";
  updateFilters(); els.search.focus();
}

function selectItem(id, moveFocus = false) {
  const item = intelligence.find((entry) => entry.id === id);
  if (!item) {
    state.selectedId = id;
    renderFeed(); renderDetail();
    if (window.matchMedia("(max-width: 1023px)").matches) {
      els.feedView.classList.add("mobile-detail");
      els.detailPane.scrollTop = 0; els.detailPane.focus();
    }
    return;
  }
  state.selectedId = id;
  renderFeed(); renderDetail();
  if (window.matchMedia("(max-width: 1023px)").matches) {
    els.feedView.classList.add("mobile-detail");
    els.detailPane.scrollTop = 0;
    els.detailPane.focus();
  } else if (moveFocus) {
    const target = els.feedList.querySelector(`[data-id="${id}"]`);
    if (target) target.focus();
  } else {
    els.detailPane.scrollTop = 0;
  }
}

function toggleAction(kind) {
  const item = selectedItem(); if (!item) return;
  const activeAction = document.activeElement?.dataset?.action;
  const activeItemId = document.activeElement?.dataset?.id;
  const collection = kind === "bookmark" ? state.bookmarked : state.research;
  const wasActive = collection.has(item.id);
  wasActive ? collection.delete(item.id) : collection.add(item.id);
  renderFeed(); renderDetail();
  if (activeAction) els.detail.querySelector(`[data-action="${activeAction}"]`)?.focus();
  else if (activeItemId) els.feedList.querySelector(`[data-id="${activeItemId}"]`)?.focus();
  state.lastUndo = () => { wasActive ? collection.add(item.id) : collection.delete(item.id); renderFeed(); renderDetail(); };
  showSnackbar(`${kind === "bookmark" ? "收藏" : "待研究"}状态已${wasActive ? "取消" : "更新"}：${item.title}`, true);
}

function showSnackbar(message, undo = false) {
  els.snackbarMessage.textContent = message;
  els.snackbarAction.hidden = !undo;
  els.snackbar.hidden = false;
  window.clearTimeout(showSnackbar.timer);
  scheduleSnackbarDismiss();
}

function scheduleSnackbarDismiss() {
  window.clearTimeout(showSnackbar.timer);
  showSnackbar.timer = window.setTimeout(() => { els.snackbar.hidden = true; state.lastUndo = null; }, 6500);
}

function openModal({ title, body, actions, trigger }) {
  state.modalTrigger = trigger || document.activeElement;
  state.modalItemId = state.selectedId;
  els.modalTitle.textContent = title;
  els.modalBody.innerHTML = `<div class="modal-body">${body}</div>`;
  els.modalActions.innerHTML = actions;
  els.modalBackdrop.hidden = false;
  document.querySelector(".app-shell").inert = true;
  document.body.style.overflow = "hidden";
  window.setTimeout(() => (els.modal.querySelector("input, button") || els.modal).focus(), 0);
}

function closeModal(preserveFeedbackDraft = true) {
  if (preserveFeedbackDraft && state.modalItemId) {
    const draft = els.modal.querySelector('input[name="feedback"]:checked')?.value;
    if (draft) state.feedbackDrafts.set(state.modalItemId, draft);
  }
  els.modalBackdrop.hidden = true;
  document.querySelector(".app-shell").inert = false;
  document.body.style.overflow = "";
  const trigger = state.modalTrigger;
  state.modalTrigger = null;
  state.modalItemId = null;
  if (trigger && document.contains(trigger)) trigger.focus();
}

function openFeedback(trigger) {
  const current = state.feedbackDrafts.get(state.selectedId) || state.feedback.get(state.selectedId) || "有价值";
  openModal({ title: "反馈这条情报", trigger,
    body: `<p>反馈只保存在当前页面内存，不会静默修改规则。</p><div class="choice-list">${["有价值", "误报", "需补充来源"].map((label) => `<label><input type="radio" name="feedback" value="${label}" ${label === current ? "checked" : ""}> ${label}</label>`).join("")}</div>`,
    actions: '<button class="button quiet" type="button" data-modal-action="cancel">取消</button><button class="button primary" type="button" data-modal-action="submit-feedback">提交反馈</button>' });
}

function openOriginal(trigger) {
  const item = selectedItem();
  if (!item) { renderDetail(); return; }
  openModal({ title: "模拟原文边界", trigger,
    body: `<p><strong>${escapeHtml(item.title)}</strong></p><p>原型不会打开网络或调用真实来源。这里仅验证“打开原文”入口和返回焦点。</p><p class="meta">来源标识：${escapeHtml(item.publisher)} · ${escapeHtml(item.originalTitle)}</p>`,
    actions: '<button class="button primary" type="button" data-modal-action="cancel">返回详情</button>' });
}

function renderRulePreview() {
  const keywords = $("rule-keywords").value.split(/[,，]/).map((v) => v.trim()).filter(Boolean);
  const exclusions = $("rule-exclusions").value.split(/[,，]/).map((v) => v.trim()).filter(Boolean);
  const threshold = Number($("rule-threshold").value);
  $("threshold-output").value = String(threshold);
  const normalizeTerm = (value) => value.normalize("NFKC").toLocaleLowerCase("zh-CN");
  const excludedTerms = new Set(exclusions.map(normalizeTerm));
  const conflict = keywords.find((word) => excludedTerms.has(normalizeTerm(word)));
  const keywordError = keywords.length ? "" : "至少需要一个关注关键词。";
  const exclusionError = conflict ? `“${conflict}”同时出现在关注与排除词中。` : "";
  setFieldError("rule-keywords", "keyword-error", keywordError);
  setFieldError("rule-exclusions", "exclusion-error", exclusionError);
  $("impact-summary").textContent = `${keywords.length} 个关注词、${exclusions.length} 个排除词、阈值 ${threshold}`;
  $("impact-count").textContent = `每日约 ${Math.max(1, Math.round((100 - threshold) / 6))} 条`;
  $("impact-conflict").textContent = conflict ? `阻断冲突：${conflict}` : "未发现";
  const risky = threshold > 88;
  $("rule-warning").hidden = !risky;
  if (!risky) $("confirm-risk").checked = false;
  $("save-rules").disabled = Boolean(keywordError || exclusionError || (risky && !$("confirm-risk").checked));
  return { keywords: $("rule-keywords").value, exclusions: $("rule-exclusions").value, threshold, invalidId: keywordError ? "rule-keywords" : exclusionError ? "rule-exclusions" : null };
}

function setFieldError(inputId, errorId, message) {
  const input = $(inputId); const error = $(errorId);
  input.setAttribute("aria-invalid", message ? "true" : "false");
  error.textContent = message; error.hidden = !message;
}

function resetRules() {
  $("rule-keywords").value = state.savedRules.keywords;
  $("rule-exclusions").value = state.savedRules.exclusions;
  $("rule-threshold").value = state.savedRules.threshold;
  $("simulate-save-failure").checked = false;
  $("rule-save-status").textContent = "修改已撤销，恢复到最近保存的规则。";
  renderRulePreview();
}

function saveRules(event) {
  event.preventDefault();
  const next = renderRulePreview();
  if (next.invalidId) { $(next.invalidId).focus(); return; }
  const button = $("save-rules");
  const form = $("rule-form");
  const controls = [...form.querySelectorAll("input, button")];
  const shouldFail = $("simulate-save-failure").checked;
  const focusBeforeSave = document.activeElement;
  const scrollBeforeSave = { x: window.scrollX, y: window.scrollY };
  controls.forEach((control) => { control.disabled = true; });
  button.textContent = "保存中…"; form.setAttribute("aria-busy", "true");
  window.setTimeout(() => {
    controls.forEach((control) => { control.disabled = false; });
    button.textContent = "保存规则"; form.removeAttribute("aria-busy");
    if (shouldFail) {
      $("rule-save-status").textContent = "规则保存失败（模拟）：输入已完整保留。关闭故障开关后可重试。";
      if (focusBeforeSave && document.contains(focusBeforeSave)) focusBeforeSave.focus();
      window.scrollTo(scrollBeforeSave.x, scrollBeforeSave.y);
    } else {
      state.savedRules = next; state.rulesPending = true;
      $("rule-save-status").textContent = "规则已保存，将在下一轮模拟同步生效；历史情报未被改写。";
      showSnackbar("规则已保存：下一轮模拟同步生效");
    }
    renderRulePreview();
  }, 450);
}

const scenarioDetails = {
  healthy: { title: "全部成功", body: "4 个来源完成；AI 队列为空。", safety: "缓存与元数据完整。", tone: "", sources: [["Official Feed", "成功 · 4 条"], ["Vendor RSS", "成功 · 2 条"], ["Policy Feed", "成功 · 1 条"]] },
  offline: { title: "离线可用", body: "网络不可用，外部动作暂停；继续展示 4 条缓存情报。", safety: "数据未丢失，未提交任何外部请求。", tone: "warning", sources: [["全部来源", "暂停 · 等待网络恢复"], ["设备缓存", "可用 · 4 条"]] },
  partial: { title: "部分成功", body: "3 个来源成功；Vendor RSS 请求阶段失败于 09:42，影响 1 个来源。", safety: "已成功结果与缓存保持可用。", tone: "warning", sources: [["Official Feed", "成功 · 4 条"], ["Vendor RSS", "失败 · 可单独重试"], ["Policy Feed", "成功 · 1 条"]] },
  rate: { title: "Vendor RSS 同步受限", body: "请求阶段于 09:42 收到限流；仅影响 Vendor RSS。退避期间入口保持可见但不可执行，等待恢复条件满足。", safety: "没有回滚其他来源，也没有重复入库。", tone: "warning", sources: [["Vendor RSS", "限流 · 退避中"], ["其他来源", "成功结果保留"]] },
  ai: { title: "AI 等待", body: "分析队列等待恢复；原始标题、来源与必要摘录仍可用，不伪造结论。", safety: "未向真实 AI 发送数据。", tone: "warning", sources: [["来源采集", "成功 · 原始事实可用"], ["AI 队列", "等待 · 可重试"]] }
};

function renderStatus() {
  const detail = scenarioDetails[state.scenario];
  $("scenario-select").value = state.scenario;
  $("sync-health-card").className = `sync-health-card ${detail.tone}`;
  $("sync-health-card").innerHTML = `<h2>${detail.title}</h2><p>${detail.body}</p><p><strong>数据安全：</strong>${detail.safety}</p><p class="meta">恢复计数：${state.retryCount}；重复入库/分析/通知：0</p>`;
  $("source-results").innerHTML = detail.sources.map(([name, result]) => `<div class="source-row"><div><strong>${name}</strong><p class="meta">${result}</p></div><span class="badge">${result.split(" · ")[0]}</span></div>`).join("");
  const retryBlocked = state.scenario === "healthy" || state.scenario === "rate";
  $("retry-button").disabled = retryBlocked;
  $("retry-button").title = state.scenario === "rate" ? "限流退避中；等待恢复条件满足后再重试" : "";
  renderHealthBanner();
}

function retryScenario() {
  if (state.scenario === "healthy" || state.scenario === "rate") return;
  const button = $("retry-button");
  const scenarioSelect = $("scenario-select");
  button.disabled = true; scenarioSelect.disabled = true; button.textContent = "恢复中…"; $("sync-health-card").setAttribute("aria-busy", "true");
  window.setTimeout(() => {
    state.retryCount += 1; state.scenario = "healthy";
    scenarioSelect.disabled = false; button.textContent = "重试有效任务"; $("sync-health-card").removeAttribute("aria-busy");
    renderStatus(); showSnackbar("恢复完成：来源和 AI 队列已原位更新；重复副作用为 0");
  }, 550);
}

function switchView(view) {
  state.view = view;
  document.querySelectorAll(".view").forEach((section) => { section.hidden = section.id !== `view-${view}`; section.classList.toggle("is-active", section.id === `view-${view}`); });
  document.querySelectorAll(".nav-item").forEach((button) => { const active = button.dataset.view === view; button.classList.toggle("is-active", active); active ? button.setAttribute("aria-current", "page") : button.removeAttribute("aria-current"); });
  const heading = $(`${view}-heading`); if (heading) heading.focus();
}

document.querySelectorAll(".nav-item").forEach((button) => button.addEventListener("click", () => switchView(button.dataset.view)));
els.search.addEventListener("input", () => { window.clearTimeout(updateFilters.timer); updateFilters.timer = window.setTimeout(updateFilters, 150); });
els.importance.addEventListener("change", updateFilters);
els.track.addEventListener("change", updateFilters);
$("clear-search").addEventListener("click", () => { els.search.value = ""; updateFilters(); els.search.focus(); });
$("reset-filters-empty").addEventListener("click", resetFilters);
els.filters.addEventListener("click", (event) => {
  const key = event.target.closest("[data-clear]")?.dataset.clear; if (!key) return;
  if (key === "all" || key === "query") els.search.value = "";
  if (key === "all" || key === "importance") els.importance.value = "all";
  if (key === "all" || key === "track") els.track.value = "all";
  updateFilters();
});
els.feedList.addEventListener("click", (event) => { const item = event.target.closest("[data-id]"); if (item) selectItem(item.dataset.id); });
els.detail.addEventListener("click", (event) => {
  const button = event.target.closest("[data-action]"); if (!button) return;
  const actions = { bookmark: () => toggleAction("bookmark"), research: () => toggleAction("research"), feedback: () => openFeedback(button), original: () => openOriginal(button), "safe-return": () => { history.replaceState(null, "", location.pathname + location.search); selectItem(intelligence[0].id); } };
  actions[button.dataset.action]?.();
});
$("mobile-back").addEventListener("click", () => { els.feedView.classList.remove("mobile-detail"); const selected = els.feedList.querySelector(`[data-id="${state.selectedId}"]`); if (selected) selected.focus(); });
$("sync-button").addEventListener("click", () => {
  if (state.scenario === "offline") { showSnackbar("当前为离线模拟：未执行同步，缓存内容保持可用"); return; }
  const loading = $("feed-loading"); loading.hidden = false; $("sync-button").disabled = true; $("sync-button").textContent = "同步中…";
  window.setTimeout(() => { const appliedRules = state.rulesPending; state.rulesPending = false; loading.hidden = true; $("sync-button").disabled = false; $("sync-button").textContent = "模拟同步"; $("last-sync").textContent = "最后同步：刚刚（模拟）"; showSnackbar(`${state.scenario === "healthy" ? "模拟同步完成" : "模拟同步部分完成"}：列表上下文保持不变${appliedRules ? "；新规则已在本轮生效" : ""}`); }, 550);
});
$("modal-close").addEventListener("click", closeModal);
els.modalBackdrop.addEventListener("click", (event) => { if (event.target === els.modalBackdrop) closeModal(); });
els.modalActions.addEventListener("click", (event) => {
  const action = event.target.closest("[data-modal-action]")?.dataset.modalAction; if (!action) return;
  if (action === "submit-feedback") {
    const value = els.modal.querySelector('input[name="feedback"]:checked')?.value;
    if (value) { const id = state.modalItemId; const previous = state.feedback.get(id); state.feedback.set(id, value); state.feedbackDrafts.delete(id); closeModal(false); renderDetail(); els.detail.querySelector('[data-action="feedback"]')?.focus(); state.lastUndo = () => { previous ? state.feedback.set(id, previous) : state.feedback.delete(id); renderDetail(); }; showSnackbar(`反馈已记录：${value}`, true); }
  } else closeModal();
});
els.snackbarAction.addEventListener("click", () => { if (state.lastUndo) state.lastUndo(); state.lastUndo = null; els.snackbar.hidden = true; });
els.snackbar.addEventListener("pointerenter", () => window.clearTimeout(showSnackbar.timer));
els.snackbar.addEventListener("pointerleave", scheduleSnackbarDismiss);
els.snackbar.addEventListener("focusin", () => window.clearTimeout(showSnackbar.timer));
els.snackbar.addEventListener("focusout", scheduleSnackbarDismiss);
['rule-keywords', 'rule-exclusions', 'rule-threshold'].forEach((id) => $(id).addEventListener("input", () => { $("confirm-risk").checked = false; renderRulePreview(); }));
$("confirm-risk").addEventListener("change", renderRulePreview);
$("reset-rules").addEventListener("click", resetRules);
$("rule-form").addEventListener("submit", saveRules);
$("scenario-select").addEventListener("change", (event) => { state.scenario = event.target.value; renderStatus(); });
$("retry-button").addEventListener("click", retryScenario);

document.addEventListener("keydown", (event) => {
  if (!els.modalBackdrop.hidden) {
    if (event.key === "Escape") { event.preventDefault(); closeModal(); return; }
    if (event.key === "Tab") {
      const focusable = [...els.modal.querySelectorAll('button:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])')];
      if (!focusable.length) return;
      const first = focusable[0], last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
      else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
    }
    return;
  }
  const target = event.target;
  if (target.matches("input, textarea, select, [contenteditable='true']")) return;
  if (event.key === "Escape" && els.feedView.classList.contains("mobile-detail")) { $("mobile-back").click(); return; }
  if (state.view !== "feed") return;
  const items = filteredItems();
  const selectedIndex = items.findIndex((item) => item.id === state.selectedId);
  const index = selectedIndex >= 0 ? selectedIndex : (event.key === "ArrowUp" || event.key === "k" || event.key === "K" ? 0 : -1);
  if (["j", "J", "ArrowDown", "k", "K", "ArrowUp"].includes(event.key) && items.length) {
    event.preventDefault(); const delta = ["j", "J", "ArrowDown"].includes(event.key) ? 1 : -1;
    selectItem(items[(index + delta + items.length) % items.length].id, true);
  } else if (event.key === "Enter" && items.length) { event.preventDefault(); selectItem(selectedIndex >= 0 ? state.selectedId : items[0].id, true); }
  else if (["s", "S"].includes(event.key)) { event.preventDefault(); toggleAction("bookmark"); }
  else if (["f", "F"].includes(event.key)) { event.preventDefault(); openFeedback(document.activeElement); }
  else if (["o", "O"].includes(event.key)) { event.preventDefault(); openOriginal(document.activeElement); }
  else if (event.key === "/") { event.preventDefault(); els.search.focus(); }
});

window.addEventListener("hashchange", () => {
  const match = location.hash.match(/^#item\/(.+)$/);
  if (match) {
    switchView("feed");
    try { selectItem(decodeURIComponent(match[1])); }
    catch { selectItem(match[1]); }
  }
});

renderFeed(); renderDetail(); renderRulePreview(); renderStatus();
if (location.hash) window.dispatchEvent(new Event("hashchange"));
