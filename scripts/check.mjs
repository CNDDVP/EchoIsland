import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const read = (file) => readFileSync(path.join(root, file), "utf8");
const json = (file) => JSON.parse(read(file));
function run(args) {
  const result = spawnSync(process.execPath, args, { cwd: root, stdio: "inherit" });
  if (result.error) throw result.error;
  assert.equal(result.status, 0, args.join(" "));
}
function* walk(directory) {
  for (const entry of readdirSync(path.join(root, directory), { withFileTypes: true })) {
    if (["node_modules", "target", "dist", "gen", "__pycache__"].includes(entry.name)) continue;
    const file = path.posix.join(directory, entry.name);
    if (entry.isDirectory()) yield* walk(file);
    else yield file;
  }
}

const config = json("apps/desktop/src-tauri/tauri.conf.json");
const windows = json("apps/desktop/src-tauri/tauri.windows.conf.json");
const version = /\[workspace\.package\][\s\S]*?^version = "([^"]+)"/m.exec(read("Cargo.toml"))[1];
assert.equal(config.version, version, "Tauri 与 Cargo 版本不同");
assert.equal(json("apps/desktop/package.json").version, version, "npm 与 Cargo 版本不同");
assert.equal(json("package-lock.json").packages["apps/desktop"].version, version, "npm 锁文件版本过期");
assert.deepEqual(config.plugins.updater.endpoints, ["https://github.com/CNDDVP/EchoIsland/releases/latest/download/latest.json"]);
assert.equal(config.productName, "EchoIsland", "品牌展示不应改变安装器升级身份");
assert.equal(config.identifier, "com.echoisland.desktop");
assert.equal(windows.bundle.windows.nsis.installMode, "currentUser");
assert.deepEqual(windows.bundle.windows.nsis.languages, ["SimpChinese"]);
assert.equal(windows.bundle.windows.wix.language["zh-CN"].localePath, "installer/zh-CN.wxl");
assert.match(read("apps/desktop/src-tauri/installer/main.wxs"), /InstallScope="perUser" InstallPrivileges="limited"/);
const wixZhCn = read("apps/desktop/src-tauri/installer/zh-CN.wxl");
assert.match(wixZhCn, /<String Id="LaunchApp">启动 EchoIsland<\/String>/);
assert.match(wixZhCn, /<String Id="DowngradeErrorMessage">已安装较新版本的 EchoIsland。<\/String>/);
assert.ok(read("README.md").startsWith("# EchoIsland 简体中文增强版"));

for (const file of [...walk("apps"), ...walk("crates"), ...walk("integrations"), ...walk("scripts")]) {
  if (/\.(?:mjs|cjs|js)$/.test(file)) run(["--check", file]);
  if (file.endsWith(".ts")) run(["--experimental-strip-types", "--check", file]);
  if (file.endsWith(".json")) json(file);
}
run(["scripts/check-i18n.mjs"]);
run(["--test", "integrations/tests/openclaw.test.mjs"]);
console.log("前端/脚本语法、版本、中文、更新源和集成回归检查通过。");
