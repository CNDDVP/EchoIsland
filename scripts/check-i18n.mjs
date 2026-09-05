#!/usr/bin/env node
/**
 * Validate the catalogs and inventory presentation text without modifying the checkout.
 * --json: full bilingual key/location inventory. --scan: English literal candidates for review.
 * Protocol fields, technical diagnostics, test fixtures and intentionally English documents
 * are distinguished from product text. The scan is evidence for review, not a UI/hardware test.
 */
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { t, format } from "../crates/i18n/index.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const read = (name) => readFileSync(path.join(root, name), "utf8");
const zh = JSON.parse(read("crates/i18n/locales/zh-CN.json"));
const en = JSON.parse(read("crates/i18n/locales/en-US.json"));
const placeholders = (value) => [...value.matchAll(/\{([^{}]+)\}/g)].map((m) => m[1]).sort();
assert.deepEqual(Object.keys(zh).sort(), Object.keys(en).sort(), "中英文词条必须一一对应");
for (const key of Object.keys(zh)) {
  assert.equal(typeof zh[key], "string", key);
  assert.ok(zh[key].trim(), key);
  assert.ok(en[key].trim(), key);
  assert.deepEqual(placeholders(zh[key]), placeholders(en[key]), key + " 模板参数不一致");
}
assert.equal(t("approval.required"), "需要批准");
assert.equal(t("approval.required", "en-US"), "Approval Required");
assert.equal(format("prompt.title", { source: "Codex {id}" }), "Codex {id} 需要关注");
assert.equal(format("cli.missing_value", { argument: "--bridge" }), "--bridge 后缺少参数值");
assert.match(read("apps/desktop/web/index.html"), /lang="zh-CN"/);
const directwrite = read("apps/desktop/src-tauri/src/windows_native_panel/directwrite.rs");
assert.match(directwrite, /echoisland_i18n::WINDOWS_UI_FONT/);
assert.match(directwrite, /echoisland_i18n::DEFAULT_LOCALE/);

function* files(directory) {
  for (const entry of readdirSync(path.join(root, directory), { withFileTypes: true })) {
    if (["node_modules", "target", ".git", "gen"].includes(entry.name)) continue;
    const relative = path.posix.join(directory.replaceAll("\\", "/"), entry.name);
    if (entry.isDirectory()) yield* files(relative);
    else yield relative;
  }
}
const inputs = ["apps", "crates", "integrations", "technical-notes", "scripts"]
  .flatMap((directory) => [...files(directory)]);
const references = new Map();
const candidates = [];
let productionFiles = 0;
for (const file of inputs) {
  if (!/\.(rs|mjs|cjs|js|ts|tsx|html|css|json|ps1|py|nsh|nsi|wxs|wxl|md)$/.test(file)) continue;
  if (file.startsWith("crates/i18n/locales/")) continue;
  if (/(^|\/)(tests|fixtures)(\/|\.rs)|test_fixtures|_tests\.rs/.test(file)) continue;
  let source = read(file);
  // Only the production prefix; test modules can deliberately contain English fixtures.
  if (file.endsWith(".rs")) source = source.split(/#\[cfg\(test\)\]\s*mod\s+\w+/)[0];
  const documentation = file.endsWith(".md");
  if (!documentation) productionFiles++;
  if (file !== "scripts/check-i18n.mjs") {
    const keyPattern = /(?:echoisland_i18n::(?:t|format|error)|\bt|\bformatText|\btr)\(\s*["']([a-z_]+\.[a-z_.]+)["']/g;
    for (const match of source.matchAll(keyPattern)) {
      const key = match[1];
      assert.ok(key in zh, file + " 引用了缺失词条：" + key);
      const line = source.slice(0, match.index).split("\n").length;
      const locations = references.get(key) ?? [];
      locations.push(file + ":" + line);
      references.set(key, locations);
    }
  }
  if (process.argv.includes("--scan")) {
    const lines = source.split("\n");
    for (let index = 0; index < lines.length; index++) {
      const line = lines[index].trim();
      if (/^(\/\/|\/\*|\*|#(?!\[)|;)/.test(line) || file === "scripts/check-i18n.mjs") continue;
      for (const literal of line.matchAll(/"((?:\\.|[^"\\])*)"/g)) {
        const text = literal[1];
        if (!/[A-Za-z].* /.test(text)) continue;
        if (/[\u3400-\u9fff]/u.test(text)) continue;
        const category = documentation ? "英文文档或代码示例" :
          /\b(?:warn|debug|info|error|trace)!|log_|diagnostic|context\(|anyhow|bail!|expect\(/.test(line) ? "技术诊断（需检查调用边界）" :
          /Application Support|^node |HTTP\/|SELECT |INSERT |CREATE |serde|rename_all|Bearer |System Events|cmd \/|powershell|Content-|codex_hooks|[A-Za-z]:\\|\/tmp\//.test(text) ? "协议、命令或标识（保留）" :
          "待人工判读";
        candidates.push({ location: file + ":" + (index + 1), text, category });
      }
    }
  }
}
const inventory = Object.keys(zh).sort().map((key) => ({
  key, locations: references.get(key) ?? ["共享词库；动态调用或预留回退"],
  english: en[key], chinese: zh[key], completed: true,
  protocol: false,
  evidence: "词条及调用静态校验；不代表系统安装器或硬件视觉实测",
}));
if (process.argv.includes("--json")) {
  process.stdout.write(JSON.stringify({ defaultLocale: "zh-CN", productionFiles, inventory, candidates }, null, 2) + "\n");
} else {
  console.log("简体中文词库检查通过：" + inventory.length + " 个中英词条，扫描 " + productionFiles + " 个生产文件。");
  console.log("保留协议字段、品牌、用户/Agent 内容、技术日志与英文文档；安装器与硬件视觉另行验证。");
  if (process.argv.includes("--scan")) for (const candidate of candidates) {
    console.log(candidate.location + " [" + candidate.category + "] " + candidate.text);
  }
}
