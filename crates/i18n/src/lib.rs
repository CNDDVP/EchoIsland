//! CN extension: presentation text only. Protocols and identifiers stay in their owning crates.
pub mod text;

use std::{collections::BTreeMap, sync::OnceLock};

pub const DEFAULT_LOCALE: &str = "zh-CN";
pub const WINDOWS_UI_FONT: &str = "Microsoft YaHei UI";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Locale {
    #[default]
    ZhCn,
    EnUs,
}

type Catalog = BTreeMap<String, String>;
static ZH_CN: OnceLock<Catalog> = OnceLock::new();
static EN_US: OnceLock<Catalog> = OnceLock::new();

fn catalog(locale: Locale) -> &'static Catalog {
    let (storage, source) = match locale {
        Locale::ZhCn => (&ZH_CN, include_str!("../locales/zh-CN.json")),
        Locale::EnUs => (&EN_US, include_str!("../locales/en-US.json")),
    };
    storage.get_or_init(|| serde_json::from_str(source).expect("validated embedded locale catalog"))
}

/// The product language is stable across machines, independent of OS language.
pub fn t(key: &str) -> &'static str {
    t_for(Locale::default(), key)
}

pub fn t_for(locale: Locale, key: &str) -> &'static str {
    catalog(locale)
        .get(key)
        .or_else(|| catalog(Locale::EnUs).get(key))
        .map(String::as_str)
        .unwrap_or("未提供翻译")
}

pub fn format(key: &str, values: &[(&str, &str)]) -> String {
    format_for(Locale::default(), key, values)
}

pub fn format_for(locale: Locale, key: &str, values: &[(&str, &str)]) -> String {
    // One pass: replacement values (including paths, IDs and literal braces) are never parsed.
    let mut remaining = t_for(locale, key);
    let mut output = String::with_capacity(remaining.len());
    while let Some(start) = remaining.find('{') {
        output.push_str(&remaining[..start]);
        let suffix = &remaining[start + 1..];
        let Some(end) = suffix.find('}') else {
            output.push_str(&remaining[start..]);
            return output;
        };
        let name = &suffix[..end];
        if let Some((_, value)) = values.iter().find(|(key, _)| *key == name) {
            output.push_str(value);
        } else {
            output.push_str(&remaining[start..start + end + 2]);
        }
        remaining = &suffix[end + 1..];
    }
    output.push_str(remaining);
    output
}

/// Keep diagnostic details in logs; expose a localized, actionable message to the user.
pub fn error(key: &str, detail: impl std::fmt::Display) -> String {
    tracing::warn!(message_key = key, error = %detail, "user operation failed");
    t(key).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogs_cover_the_same_keys_and_template_parameters() {
        let zh = catalog(Locale::ZhCn);
        let en = catalog(Locale::EnUs);
        assert_eq!(zh.keys().collect::<Vec<_>>(), en.keys().collect::<Vec<_>>());
        for (key, chinese) in zh {
            assert!(!chinese.trim().is_empty(), "{key}");
            let placeholders = |value: &str| {
                value
                    .split('{')
                    .skip(1)
                    .filter_map(|part| part.split_once('}'))
                    .map(|(name, _)| name.to_owned())
                    .collect::<Vec<_>>()
            };
            assert_eq!(placeholders(chinese), placeholders(&en[key]), "{key}");
        }
    }

    #[test]
    fn zh_cn_is_default_without_rewriting_identifiers_or_parameter_values() {
        assert_eq!(DEFAULT_LOCALE, "zh-CN");
        assert_eq!(t("approval.required"), "需要批准");
        assert_eq!(
            t_for(Locale::EnUs, "approval.required"),
            "Approval Required"
        );
        assert_eq!(
            format("cli.missing_value", &[("argument", "--bridge")]),
            "--bridge 后缺少参数值"
        );
        assert_eq!(
            format("prompt.title", &[("source", "Codex {id}")]),
            "Codex {id} 需要关注"
        );
        assert_eq!(format("display.number", &[("number", "2")]), "显示器 2");
    }
}
