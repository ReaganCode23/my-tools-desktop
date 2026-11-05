//! Opt-in disposable CI validation; excluded from distributed builds.
use crate::{vault, vault_core};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_clipboard_manager::ClipboardExt;

pub fn start_if_requested(app: AppHandle) {
    if std::env::var("MYTOOLS_VAULT_SMOKE").as_deref() != Ok("1") { return; }
    assert_eq!(std::env::var("CI").as_deref(), Ok("true"), "Vault validation requires disposable CI");
    let report = std::env::var_os("MYTOOLS_SMOKE_RESULT").expect("report path required");
    let window = WebviewWindowBuilder::new(&app, "vault", WebviewUrl::App("vault.html".into()))
        .visible(false).build().expect("test webview");
    tauri::async_runtime::spawn(async move {
        let result: Result<(), String> = async {
            let path = app.path().app_config_dir().map_err(|e|e.to_string())?.join("password-vault.mtvault");
            if path.exists() { return Err("Refusing to use an existing vault".into()); }
            let main = app.get_webview_window("main").ok_or("main window missing")?;
            assert!(vault::vault_status(app.clone(),main.clone(),app.state()).is_err());
            assert!(vault::vault_unlock(app.clone(),main,"test master password 2026".into()).await.is_err());
            let password = "test master password 2026";
            vault::vault_create(app.clone(),window.clone(),password.into()).await?;
            let id = vault::vault_save(app.clone(),window.clone(),vault::Input {
                id:None,title:"CI Account".into(),username:"ci@example.invalid".into(),
                url:"https://example.invalid".into(),notes:"private CI note".into(),
                folder:"Tests".into(),password:Some("CI-secret-unique-93!".into()),favourite:true
            }).await?;
            let summaries=serde_json::to_string(&vault::vault_list(window.clone(),app.state())?).unwrap();
            assert!(!summaries.contains("CI-secret-unique-93!"));
            let details=serde_json::to_string(&vault::vault_get(window.clone(),app.state(),id.clone())?).unwrap();
            assert!(!details.contains("CI-secret-unique-93!"));
            assert_eq!(vault::vault_reveal(window.clone(),app.state(),id.clone())?,"CI-secret-unique-93!");
            assert!(!String::from_utf8_lossy(&vault_core::read(&path)?).contains("CI-secret-unique-93!"));
            vault::vault_lock(app.clone(),window.clone())?;
            assert!(vault::vault_reveal(window.clone(),app.state(),id.clone()).is_err());
            assert!(vault::vault_unlock(app.clone(),window.clone(),"wrong password".into()).await.is_err());
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            vault::vault_unlock(app.clone(),window.clone(),password.into()).await?;
            vault::vault_archive(app.clone(),window.clone(),id.clone(),true).await?;
            assert!(vault::vault_reveal(window.clone(),app.state(),id.clone()).is_err());
            vault::vault_archive(app.clone(),window.clone(),id.clone(),false).await?;
            assert!(vault::vault_change_password(app.clone(),window.clone(),"replacement master password".into(),"wrong password".into()).await.is_err());
            vault::vault_change_password(app.clone(),window.clone(),"replacement master password".into(),password.into()).await?;
            assert!(vault::vault_unlock(app.clone(),window.clone(),password.into()).await.is_err());
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            vault::vault_unlock(app.clone(),window.clone(),"replacement master password".into()).await?;
            assert_eq!(vault::vault_reveal(window.clone(),app.state(),id.clone())?,"CI-secret-unique-93!");
            vault::vault_copy(app.clone(),window.clone(),app.state(),id.clone())?;
            assert_eq!(app.clipboard().read_text().map_err(|e|e.to_string())?,"CI-secret-unique-93!");
            vault::vault_lock(app.clone(),window.clone())?;
            assert!(app.clipboard().read_text().unwrap_or_default()!="CI-secret-unique-93!");
            vault::vault_unlock(app.clone(),window.clone(),"replacement master password".into()).await?;
            vault::blur_lock(&app);
            assert!(vault::vault_reveal(window,app.state(),id).is_err());
            Ok(())
        }.await;
        let status=if result.is_ok(){"passed"}else{"failed"};
        let _=std::fs::write(report,serde_json::json!({"status":status,"error":result.err(),"checks":["window isolation","encrypted persistence","lock and unlock","wrong passwords","trash and restore","master password rotation","clipboard clearing","focus loss locking"]}).to_string());
        app.exit(if status=="passed"{0}else{1});
    });
}
