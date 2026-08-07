//! Chat page and server procedure that talks to the configured LLM.

#![allow(clippy::too_many_lines)]

use std::time::{SystemTime, UNIX_EPOCH};

use topcoat::Result;
use topcoat::router::page;
use topcoat::runtime::procedure;
use topcoat::view::{component, view};

use crate::app::{
    character_chat as app_character_chat, character_create as app_character_create,
    character_delete as app_character_delete, character_list as app_character_list,
    character_regenerate as app_character_regenerate, format_character_list_summary,
    prompt_message,
};

/// UTC `HH:MM:SS` for message lines (no extra time crate).
fn utc_hms() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    format!("{h:02}:{m:02}:{s:02}")
}

/// Server-side chat turn. Always returns `Ok(String)` so the browser only
/// ever sees a `StringSurrogate` (Topcoat 0.5 has no `is_ok` / `unwrap` on it).
/// Reply is prefixed with a UTC timestamp for the log.
#[procedure]
async fn send_chat(message: String) -> Result<String> {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return Ok(format!("[{}] (empty message)", utc_hms()));
    }
    match prompt_message(trimmed).await {
        Ok(reply) => Ok(format!("[{}] {reply}", utc_hms())),
        Err(err) => Ok(format!("[{}] (error: {err})", utc_hms())),
    }
}

/// Local RPC wrapper for the in-page character-list button.
///
/// `#[procedure]` makes the function reference visible to the Topcoat
/// `view!` parser (which transforms every call into `name.call((args))` on
/// the `&Procedure<…>` const). The plain `app::character_list` returns
/// `Result<Vec<CharacterSummary>>` — `CharacterSummary` is not a
/// `Surrogated` type, so the view! machinery rejects it. The wrapper hides
/// the typed list behind a `String` (the formatted text) so the view!
/// closure can call it via the same RPC path as `send_chat`.
#[procedure]
pub async fn ui_character_list() -> Result<String> {
    match app_character_list() {
        Ok(list) => Ok(format!(
            "[{}] {}",
            utc_hms(),
            format_character_list_summary(&list)
        )),
        Err(err) => Ok(format!("[{}] (error: {err})", utc_hms())),
    }
}

/// Local RPC wrapper for the create-character form.
#[procedure]
pub async fn ui_character_create(concept: String) -> Result<String> {
    match app_character_create(&concept).await {
        Ok(summary) => Ok(format!("[{}] {summary}", utc_hms())),
        Err(err) => Ok(format!("[{}] (error: {err})", utc_hms())),
    }
}

/// Local RPC wrapper for the character-chat form.
#[procedure]
pub async fn ui_character_chat(slug: String, message: String) -> Result<String> {
    match app_character_chat(&slug, &message).await {
        Ok(reply) => Ok(format!("[{}] {reply}", utc_hms())),
        Err(err) => Ok(format!("[{}] (error: {err})", utc_hms())),
    }
}

/// Local RPC wrapper for the delete-character form.
#[procedure]
pub async fn ui_character_delete(slug: String) -> Result<String> {
    match app_character_delete(&slug) {
        Ok(summary) => Ok(format!("[{}] {summary}", utc_hms())),
        Err(err) => Ok(format!("[{}] (error: {err})", utc_hms())),
    }
}

/// Local RPC wrapper for the regenerate-character form.
#[procedure]
pub async fn ui_character_regenerate(slug: String) -> Result<String> {
    match app_character_regenerate(&slug).await {
        Ok(summary) => Ok(format!("[{}] {summary}", utc_hms())),
        Err(err) => Ok(format!("[{}] (error: {err})", utc_hms())),
    }
}

#[page("/")]
async fn home() -> Result {
    view! {
        <!DOCTYPE html>
        <html lang="zh-CN">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>"NovelAgent Chat"</title>
                topcoat::runtime::script()
                topcoat::dev::script()
                <script>
                    "(function () {"
                    "  function scroll() {"
                    "    document.querySelectorAll('.log').forEach(function (el) {"
                    "      el.scrollTop = el.scrollHeight;"
                    "    });"
                    "  }"
                    "  function setup() {"
                    "    var obs = new MutationObserver(scroll);"
                    "    obs.observe(document.body, { childList: true, characterData: true, subtree: true });"
                    "    scroll();"
                    "  }"
                    "  window.addEventListener('load', setup);"
                    "  document.addEventListener('DOMContentLoaded', setup);"
                    "  setTimeout(scroll, 50);"
                    "})();"
                </script>
                <style>
                    "*, *::before, *::after { box-sizing: border-box; }"
                    "body { margin: 0; font-family: system-ui, -apple-system, 'PingFang SC', 'Microsoft YaHei', sans-serif; background: #0f1419; color: #e7e9ea; min-height: 100vh; }"
                    ".shell { max-width: 720px; margin: 0 auto; padding: 1.5rem 1rem 2rem; display: flex; flex-direction: column; gap: 1rem; min-height: 100vh; }"
                    "header h1 { margin: 0 0 0.25rem; font-size: 1.35rem; font-weight: 650; }"
                    "header p { margin: 0; color: #8b98a5; font-size: 0.9rem; }"
                    ".card { background: #16181c; border: 1px solid #2f3336; border-radius: 12px; padding: 1rem; display: flex; flex-direction: column; gap: 0.75rem; }"
                    ".card h2 { margin: 0; font-size: 0.95rem; font-weight: 600; color: #c9d1d9; display: flex; align-items: baseline; gap: 0.5rem; }"
                    ".card h2 .hint { color: #6b7280; font-weight: 400; font-size: 0.78rem; }"
                    ".row { display: flex; gap: 0.5rem; align-items: center; }"
                    ".row .slug-input { flex: 1; }"
                    ".row .clear-btn { background: transparent; color: #6b7280; font-weight: 400; font-size: 0.78rem; padding: 0.25rem 0.6rem; min-width: 0; border-radius: 999px; }"
                    ".row .clear-btn:hover:not(:disabled) { background: #1f2429; color: #c9d1d9; }"
                    ".log { background: #0f1419; border: 1px solid #2f3336; border-radius: 8px; padding: 0.75rem; white-space: pre-wrap; word-break: break-word; min-height: 80px; max-height: 320px; overflow-y: auto; line-height: 1.55; font-size: 0.92rem; font-family: ui-monospace, SFMono-Regular, 'Cascadia Code', Consolas, monospace; }"
                    ".composer { display: flex; gap: 0.5rem; }"
                    ".composer input, .row input { flex: 1; border-radius: 999px; border: 1px solid #2f3336; background: #0f1419; color: #e7e9ea; padding: 0.6rem 0.9rem; font-size: 0.95rem; outline: none; transition: border-color 120ms ease; }"
                    ".composer input:focus, .row input:focus { border-color: #1d9bf0; box-shadow: 0 0 0 1px #1d9bf0; }"
                    ".composer input::placeholder, .row input::placeholder { color: #6b7280; }"
                    ".composer button, .row button { border: 0; border-radius: 999px; background: #1d9bf0; color: #fff; font-weight: 600; padding: 0.6rem 1rem; cursor: pointer; font-size: 0.92rem; min-width: 4.5rem; transition: opacity 120ms ease, background 120ms ease; }"
                    ".composer button:hover:not(:disabled), .row button:hover:not(:disabled) { background: #1a8cd8; }"
                    ".composer button:disabled, .row button:disabled { opacity: 0.55; cursor: not-allowed; }"
                    ".composer button:disabled::before { content: \"\\2022\"; display: inline-block; color: #1d9bf0; margin-right: 0.4rem; font-size: 1.1rem; line-height: 1; vertical-align: middle; animation: pulse 1.2s ease-in-out infinite; }"
                    ".row button:disabled::before { content: \"\\2022\"; display: inline-block; color: #8b98a5; margin-right: 0.4rem; font-size: 1.1rem; line-height: 1; vertical-align: middle; animation: pulse 1.2s ease-in-out infinite; }"
                    ".row button.secondary { background: #2f3336; color: #e7e9ea; }"
                    ".row button.secondary:hover:not(:disabled) { background: #3a3f44; }"
                    "@keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.4; } }"
                    ".busy-dot { display: inline-block; width: 0.45rem; height: 0.45rem; border-radius: 999px; background: #1d9bf0; margin-right: 0.4rem; vertical-align: middle; animation: pulse 1.2s ease-in-out infinite; color: transparent; font-size: 0; line-height: 0; }"
                    ".log { scroll-behavior: smooth; }"
                </style>
            </head>
            <body>
                chat_panel()
            </body>
        </html>
    }
}

#[allow(clippy::too_many_lines)]
#[component]
async fn chat_panel() -> Result {
    view! {
        signal llm_draft = String::new();
        signal llm_transcript = String::new();
        signal llm_busy = false;

        signal list_text = String::new();
        signal list_busy = false;

        signal create_draft = String::new();
        signal create_log = String::new();
        signal create_busy = false;

        signal selected_slug = String::new();
        signal chat_draft = String::new();
        signal chat_log = String::new();
        signal chat_busy = false;

        signal manage_log = String::new();
        signal manage_busy = false;

        <div class="shell">
            <header>
                <h1>"NovelAgent"</h1>
                <p>"浏览器 ↔ DeepSeek V4 Flash (OpenCode Go) · 本地角色卡管理"</p>
            </header>

            <section class="card">
                <div class="row">
                    <h2>"LLM 直接对话"</h2>
                    <button
                        class="clear-btn"
                        @click=$(async |e: topcoat::runtime::Event| {
                            e.prevent_default();
                            llm_transcript.set("".to_owned());
                        })
                    >
                        "清空"
                    </button>
                </div>
                <div class="log">$(llm_transcript.get())</div>
                <form
                    class="composer"
                    @submit=$(async |e: topcoat::runtime::Event| {
                        e.prevent_default();
                        let msg = llm_draft.get();
                        let trimmed = msg.trim();
                        if trimmed.is_empty() {
                            return;
                        }
                        llm_busy.set(true);
                        llm_transcript.push_str("\u{4f60}: ");
                        llm_transcript.push_str(trimmed);
                        llm_transcript.push_str("\n");
                        llm_draft.set("".to_owned());
                        let outcome = send_chat(msg).await;
                        llm_transcript.push_str("LLM: ");
                        llm_transcript.push_str(outcome.trim());
                        llm_transcript.push_str("\n");
                        llm_busy.set(false);
                    })
                >
                    <input
                        type="text"
                        placeholder="输入消息后按回车发送"
                        :value=$(llm_draft.get())
                        @input=$(|e: topcoat::runtime::Event| llm_draft.set(e.target.value))
                    >
                    <button
                        type="submit"
                        :disabled=$(llm_busy.get())
                    >"发送"
                    </button>
                </form>
            </section>

            <section class="card">
                <div class="row">
                    <h2>
                        "已存角色"
                        <span class="hint">"data/characters/"</span>
                    </h2>
                    <button
                        class="clear-btn"
                        @click=$(async |e: topcoat::runtime::Event| {
                            e.prevent_default();
                            list_text.set("".to_owned());
                        })
                    >
                        "清空"
                    </button>
                    <button
                        class="secondary"
                        @click=$(async |e: topcoat::runtime::Event| {
                            e.prevent_default();
                            list_busy.set(true);
                            let outcome = ui_character_list().await;
                            if list_text.get().is_empty() == false {
                                list_text.push_str("\n");
                            }
                            list_text.push_str(outcome.trim());
                            list_text.push_str("\n");
                            list_busy.set(false);
                        })
                        :disabled=$(list_busy.get())
                    >
                        "刷新"
                    </button>
                </div>
                <div class="log">$(list_text.get())</div>
            </section>

            <section class="card">
                <div class="row">
                    <h2>
                        "创建新角色"
                        <span class="hint">"Self-Refine 多轮生成"</span>
                    </h2>
                    <button
                        class="clear-btn"
                        @click=$(async |e: topcoat::runtime::Event| {
                            e.prevent_default();
                            create_log.set("".to_owned());
                        })
                    >
                        "清空"
                    </button>
                </div>
                <form
                    class="composer"
                    @submit=$(async |e: topcoat::runtime::Event| {
                        e.prevent_default();
                        let concept = create_draft.get();
                        let trimmed = concept.trim();
                        if trimmed.is_empty() {
                            return;
                        }
                        create_busy.set(true);
                        create_log.push_str("\n[create] ");
                        create_log.push_str(trimmed);
                        create_log.push_str("\n");
                        create_draft.set("".to_owned());
                        let outcome = ui_character_create(concept).await;
                        create_log.push_str(outcome.trim());
                        create_log.push_str("\n");
                        create_busy.set(false);
                    })
                >
                    <input
                        type="text"
                        placeholder="一句话概念, 例: 夜班书店的沉默女店员"
                        :value=$(create_draft.get())
                        @input=$(|e: topcoat::runtime::Event| create_draft.set(e.target.value))
                    >
                    <button
                        type="submit"
                        :disabled=$(create_busy.get())
                    >"创建"
                    </button>
                </form>
                <div class="log">$(create_log.get())</div>
            </section>

            <section class="card">
                <div class="row">
                    <h2>
                        "管理"
                        <span class="hint">"按 slug 删除 / 重新生成"</span>
                    </h2>
                    <button
                        class="clear-btn"
                        @click=$(async |e: topcoat::runtime::Event| {
                            e.prevent_default();
                            manage_log.set("".to_owned());
                        })
                    >
                        "清空"
                    </button>
                </div>
                <div class="row">
                    <input
                        class="slug-input"
                        type="text"
                        placeholder="slug (从上面列表复制, 例: 苏晚)"
                        :value=$(selected_slug.get())
                        @input=$(|e: topcoat::runtime::Event| selected_slug.set(e.target.value))
                    >
                </div>
                <div class="row">
                    <button
                        class="secondary"
                        @click=$(async |e: topcoat::runtime::Event| {
                            e.prevent_default();
                            let slug = selected_slug.get();
                            let slug_trimmed = slug.trim();
                            if slug_trimmed.is_empty() {
                                return;
                            }
                            manage_busy.set(true);
                            let outcome = ui_character_delete(slug.clone()).await;
                            manage_log.push_str("\n[delete] ");
                            manage_log.push_str(slug_trimmed);
                            manage_log.push_str("\n");
                            manage_log.push_str(outcome.trim());
                            manage_log.push_str("\n");
                            manage_busy.set(false);
                        })
                        :disabled=$(manage_busy.get())
                    >
                        "删除"
                    </button>
                    <button
                        class="secondary"
                        @click=$(async |e: topcoat::runtime::Event| {
                            e.prevent_default();
                            let slug = selected_slug.get();
                            let slug_trimmed = slug.trim();
                            if slug_trimmed.is_empty() {
                                return;
                            }
                            manage_busy.set(true);
                            let outcome = ui_character_regenerate(slug.clone()).await;
                            manage_log.push_str("\n[regenerate] ");
                            manage_log.push_str(slug_trimmed);
                            manage_log.push_str("\n");
                            manage_log.push_str(outcome.trim());
                            manage_log.push_str("\n");
                            manage_busy.set(false);
                        })
                        :disabled=$(manage_busy.get())
                    >
                        "重新生成"
                    </button>
                </div>
                <div class="log">$(manage_log.get())</div>
            </section>

            <section class="card">
                <div class="row">
                    <h2>
                        "角色对话"
                        <span class="hint">"prompt pack · 检索 + 生成"</span>
                    </h2>
                    <button
                        class="clear-btn"
                        @click=$(async |e: topcoat::runtime::Event| {
                            e.prevent_default();
                            chat_log.set("".to_owned());
                        })
                    >
                        "清空"
                    </button>
                </div>
                <div class="row">
                    <input
                        class="slug-input"
                        type="text"
                        placeholder="角色 slug (从上面复制, 例: 苏晚)"
                        :value=$(selected_slug.get())
                        @input=$(|e: topcoat::runtime::Event| selected_slug.set(e.target.value))
                    >
                </div>
                <div class="log">$(chat_log.get())</div>
                <form
                    class="composer"
                    @submit=$(async |e: topcoat::runtime::Event| {
                        e.prevent_default();
                        let slug = selected_slug.get();
                        let slug_trimmed = slug.trim();
                        let msg = chat_draft.get();
                        let msg_trimmed = msg.trim();
                        if slug_trimmed.is_empty() {
                            return;
                        }
                        if msg_trimmed.is_empty() {
                            return;
                        }
                        chat_busy.set(true);
                        chat_log.push_str("\n[");
                        chat_log.push_str(slug_trimmed);
                        chat_log.push_str("] 你: ");
                        chat_log.push_str(msg_trimmed);
                        chat_log.push_str("\n");
                        chat_draft.set("".to_owned());
                        let outcome = ui_character_chat(slug.clone(), msg).await;
                        chat_log.push_str("[");
                        chat_log.push_str(slug_trimmed);
                        chat_log.push_str("] Card: ");
                        chat_log.push_str(outcome.trim());
                        chat_log.push_str("\n");
                        chat_busy.set(false);
                    })
                >
                    <input
                        type="text"
                        placeholder="对角色说点什么…"
                        :value=$(chat_draft.get())
                        @input=$(|e: topcoat::runtime::Event| chat_draft.set(e.target.value))
                    >
                    <button
                        type="submit"
                        :disabled=$(chat_busy.get())
                    >"发送"
                    </button>
                </form>
            </section>
        </div>
    }
}
