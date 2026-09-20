//! OpenClaude Desktop — unofficial open-source Claude desktop client.
//!
//! Layering, outermost first:
//!
//! ```text
//!   React UI  ──IPC──▶  commands/  ──▶  chat / db::repo / attachments
//!                                        │        │            │
//!                                   provider/   db/        blob store
//!                                        │        │
//!                              `claude` CLI    SQLite
//!                              (subprocess)
//! ```
//!
//! The renderer has no network access, and neither does this crate: every
//! request is made by the Claude Code CLI, spawned as a child process.

pub mod attachments;
pub mod chat;
pub mod commands;
pub mod db;
pub mod error;
pub mod export;
pub mod mcp;
pub mod paths;
pub mod provider;
pub mod quick_chat;
pub mod settings_defaults;
pub mod state;
pub mod tray;

use db::Db;
use state::AppState;
use std::sync::Arc;
use tauri::Manager;

/// Extra diagnostics, enabled with `OPENCLAUDE_DEV=1`.
/// Never changes what is logged about message *content*.
pub fn dev_mode() -> bool {
    matches!(
        std::env::var("OPENCLAUDE_DEV").as_deref(),
        Ok("1") | Ok("true")
    )
}

fn init_logging() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let default = if dev_mode() {
        "openclaude=debug,warn"
    } else {
        "openclaude=info,warn"
    };
    let filter =
        EnvFilter::try_from_env("OPENCLAUDE_LOG").unwrap_or_else(|_| EnvFilter::new(default));

    // stderr only. Prompt and response text is never passed to a logging
    // macro anywhere in this crate; see docs/SECURITY-MODEL.md.
    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(false).with_writer(std::io::stderr))
        .try_init();
}

/// Housekeeping that must happen before the window is shown.
fn on_startup(db: &Db) -> error::Result<()> {
    let conn = db.conn();

    // Anything still marked in-flight belonged to a previous process.
    db::repo::messages::recover_in_flight(&conn)?;

    // Attachments staged but never sent are dead weight after a restart.
    let day_ago = db::models::now_ms() - 24 * 60 * 60 * 1000;
    db::repo::attachments::delete_dangling(&conn, day_ago)?;

    // Age out the trash according to the configured retention.
    let days: i64 = db::repo::settings::get_or(&conn, settings_defaults::TRASH_RETENTION_DAYS, 30);
    if days > 0 {
        let cutoff = db::models::now_ms() - days * 24 * 60 * 60 * 1000;
        db::repo::conversations::purge_trash_older_than(&conn, cutoff)?;
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logging();

    if let Err(e) = paths::ensure_dirs() {
        eprintln!("OpenClaude: cannot create its data directories: {e}");
        std::process::exit(1);
    }

    let db_path = paths::database_path();
    let db = match Db::open(&db_path) {
        Ok(db) => Arc::new(db),
        Err(e) => {
            // A corrupt or unreadable database is the one failure worth a
            // native dialog: the window would otherwise open into nothing.
            eprintln!("OpenClaude: cannot open {}: {e}", db_path.display());
            std::process::exit(1);
        }
    };

    if let Err(e) = on_startup(&db) {
        tracing::warn!(error = %e, "startup housekeeping failed");
    }

    let state = Arc::new(AppState::new(db));

    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(state.clone())
        .setup({
            let state = state.clone();
            move |app| {
                // The window is created hidden and revealed here, after
                // window-state has restored its geometry — otherwise it visibly
                // jumps from the default size to the saved one on every launch.
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                }
                crate::tray::install(app.handle(), &state);
                crate::quick_chat::install(app.handle(), &state);
                tracing::info!(
                    version = env!("CARGO_PKG_VERSION"),
                    "OpenClaude Desktop started"
                );
                Ok(())
            }
        })
        .on_window_event({
            let state = state.clone();
            move |window, event| match event {
                // With a tray, closing means hide: the app keeps running and
                // generations in flight are not interrupted.
                tauri::WindowEvent::CloseRequested { api, .. }
                    if crate::tray::close_to_tray(&state) =>
                {
                    api.prevent_close();
                    let _ = window.hide();
                }
                tauri::WindowEvent::Destroyed => {
                    // Let running generations finalise their rows rather than
                    // leaving them to be recovered on next launch.
                    state.cancel_all();
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            // conversations
            commands::conversations::list_conversations,
            commands::conversations::get_conversation,
            commands::conversations::create_conversation,
            commands::conversations::get_messages,
            commands::conversations::rename_conversation,
            commands::conversations::set_conversation_model,
            commands::conversations::set_conversation_pinned,
            commands::conversations::set_conversation_archived,
            commands::conversations::set_conversation_project,
            commands::conversations::set_conversation_system_prompt,
            commands::conversations::trash_conversation,
            commands::conversations::restore_conversation,
            commands::conversations::delete_conversation_permanently,
            commands::conversations::empty_trash,
            commands::conversations::duplicate_conversation,
            commands::conversations::branch_conversation,
            commands::conversations::conversation_usage,
            commands::conversations::export_conversation,
            commands::conversations::write_text_file,
            commands::conversations::delete_message,
            // chat
            commands::chat::send_message,
            commands::chat::retry_message,
            commands::chat::edit_and_resend,
            commands::chat::continue_message,
            commands::chat::stop_generation,
            commands::chat::is_generating,
            // settings & models
            commands::settings::get_settings,
            commands::settings::set_setting,
            commands::settings::set_settings,
            commands::settings::reset_setting,
            commands::settings::list_models,
            // projects
            commands::projects::list_projects,
            commands::projects::get_project,
            commands::projects::create_project,
            commands::projects::update_project,
            commands::projects::set_project_archived,
            commands::projects::delete_project,
            // search
            commands::settings::set_quick_chat_shortcut,
            commands::mcp::list_mcp_servers,
            commands::mcp::add_mcp_server,
            commands::mcp::set_mcp_server_enabled,
            commands::mcp::delete_mcp_server,
            commands::mcp::list_mcp_permissions,
            commands::mcp::decide_mcp_tool,
            commands::mcp::discover_mcp_tools,
            commands::prompts::list_prompts,
            commands::prompts::create_prompt,
            commands::prompts::update_prompt,
            commands::prompts::delete_prompt,
            commands::prompts::mark_prompt_used,
            commands::search::search_all,
            commands::search::search_conversation,
            commands::search::rebuild_search_index,
            // attachments
            commands::attachments::add_attachments,
            commands::attachments::add_image_attachment,
            commands::attachments::remove_attachment,
            commands::attachments::list_attachments,
            commands::attachments::read_attachment_data_url,
            // system
            commands::system::app_info,
            commands::system::claude_code_status,
            commands::system::storage_stats,
            commands::system::backup_database,
            commands::system::vacuum_database,
            commands::system::clear_all_conversations,
            commands::system::export_all_data,
            commands::system::prune_orphan_attachments,
        ])
        .run(tauri::generate_context!())
        .expect("failed to start OpenClaude Desktop");
}
