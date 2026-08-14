import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = dirname(fileURLToPath(import.meta.url));
const files = Object.fromEntries(await Promise.all(
  ["index.html", "styles.css", "app.js", "README.md"].map(async (name) => [name, await readFile(join(root, name), "utf8")])
));

const checks = [
  ["HTML 声明中文与移动视口", /lang="zh-CN"/.test(files["index.html"]) && /name="viewport"/.test(files["index.html"])],
  ["原型边界可见", files["index.html"].includes("不访问网络")],
  ["情报列表与详情语义存在", /role="listbox"/.test(files["index.html"]) && /detail-pane/.test(files["index.html"])],
  ["规则与诊断区域存在", /view-rules/.test(files["index.html"]) && /view-status/.test(files["index.html"])],
  ["对话框具备模态语义", /aria-modal="true"/.test(files["index.html"])],
  ["焦点与减弱动画规则存在", /:focus-visible/.test(files["styles.css"]) && /prefers-reduced-motion/.test(files["styles.css"])],
  ["390px 单栈断点存在", /max-width:\s*639px/.test(files["styles.css"]) && /mobile-detail/.test(files["styles.css"])],
  ["设计令牌通过 CSS 变量消费", /--primary:/.test(files["styles.css"]) && /var\(--primary\)/.test(files["styles.css"])],
  ["核心交互状态存在", /bookmarked:\s*new Set/.test(files["app.js"]) && /feedback:\s*new Map/.test(files["app.js"])],
  ["规则校验与恢复计数存在", /renderRulePreview/.test(files["app.js"]) && /retryCount/.test(files["app.js"])],
  ["深链解析与手机安全回退有守卫", /try\s*\{\s*selectItem\(decodeURIComponent/.test(files["app.js"]) && /mobile-detail/.test(files["app.js"])],
  ["覆盖层隔离背景并恢复焦点", /\.inert\s*=\s*true/.test(files["app.js"]) && /modalItemId/.test(files["app.js"])],
  ["异步规则保存锁定控件", /controls\.forEach\(\(control\).*disabled\s*=\s*true/.test(files["app.js"])],
  ["健康状态禁止伪恢复", /state\.scenario\s*===\s*"healthy"/.test(files["app.js"])],
  ["无已知网络调用或外部资源", !/\bfetch\s*\(|XMLHttpRequest|WebSocket\s*\(|EventSource\s*\(|sendBeacon\s*\(|https?:\/\//i.test(files["app.js"] + files["index.html"] + files["styles.css"]) && !/url\s*\(/i.test(files["styles.css"])],
  ["人工验收双视口有说明", files["README.md"].includes("390px") && files["README.md"].includes("桌面")]
];

const failed = checks.filter(([, ok]) => !ok);
for (const [name, ok] of checks) console.log(`${ok ? "PASS" : "FAIL"}  ${name}`);
if (failed.length) {
  console.error(`\n${failed.length} 项静态契约检查失败。`);
  process.exitCode = 1;
} else {
  console.log(`\n${checks.length} 项静态契约检查全部通过。`);
}
