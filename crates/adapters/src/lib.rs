mod adapter;
pub mod claude;
pub mod codex;
pub mod feishu;
mod install_support;
pub mod openclaw;
mod platform_support;

pub use adapter::{AdapterPath, AdapterStatus, InstallableAdapter, SessionScanningAdapter};
pub use claude::{
    default_paths as claude_default_paths, get_claude_status, install_claude_adapter,
    scan_claude_sessions, ClaudeAdapter, ClaudePaths, ClaudeSessionScanner, ClaudeStatus,
};
pub use codex::{
    default_paths, get_codex_status, install_codex_adapter, scan_codex_sessions, CodexAdapter,
    CodexPaths, CodexSessionScanner, CodexStatus,
};
pub use feishu::{
    envelope_from_chat_reply, envelope_from_compact_event, parse_chat_messages_response,
    parse_compact_line, should_poll_reply_for_event, FeishuChatMessage, FeishuCompactEvent,
};
pub use openclaw::{
    default_paths as openclaw_default_paths, get_openclaw_status, install_openclaw_adapter,
    OpenClawAdapter, OpenClawPaths, OpenClawStatus,
};
