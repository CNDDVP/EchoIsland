import { readFileSync } from "node:fs";

// Shared source catalog for Node build scripts and integrations.
const zh = JSON.parse(readFileSync(new URL("./locales/zh-CN.json", import.meta.url), "utf8"));
const en = JSON.parse(readFileSync(new URL("./locales/en-US.json", import.meta.url), "utf8"));
export function t(key, locale = "zh-CN") {
  return (locale === "en-US" ? en[key] : zh[key]) ?? en[key] ?? "未提供翻译";
}
export function format(key, values = {}, locale = "zh-CN") {
  return t(key, locale).replace(/\{([^{}]+)\}/g, (token, name) => values[name] ?? token);
}
