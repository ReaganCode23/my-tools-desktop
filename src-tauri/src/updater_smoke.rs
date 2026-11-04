//! Opt-in CI harness. Not compiled into distributed builds and never used on user files.
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::UpdaterExt;

pub fn start_if_requested(app: AppHandle) {
    if std::env::var("MYTOOLS_UPDATER_SMOKE").as_deref() != Ok("1") { return; }
    let report = std::env::var_os("MYTOOLS_SMOKE_RESULT").expect("smoke report path is required");
    tauri::async_runtime::spawn(async move {
        let result: Result<(), String> = async {
            // Reinstall the current published version in this disposable CI directory.
            // Production update checks retain the default strictly-newer comparator.
            let update = app.updater_builder()
                .timeout(std::time::Duration::from_secs(60))
                .version_comparator(|_, _| true)
                .build().map_err(|e| e.to_string())?
                .check().await.map_err(|e| e.to_string())?
                .ok_or("The published update feed returned no release")?;
            let version = update.version.clone();
            *app.state::<super::AppState>().pending.lock().map_err(|e| e.to_string())? = Some(update);
            // Exercise the exact download/signature verification command used by the UI.
            super::download_update(app.state()).await?;
            std::fs::write(&report, serde_json::json!({
                "status": "verified-download", "version": version,
                "executable": std::env::current_exe().map_err(|e| e.to_string())?
            }).to_string()).map_err(|e| e.to_string())?;
            // Exercise the real platform installation command, including restart on Linux.
            super::install_update(app.clone(), app.state()).await
        }.await;
        if let Err(error) = result {
            let _ = std::fs::write(&report, serde_json::json!({"status":"failed","error":error}).to_string());
            app.exit(1);
        }
    });
}
