#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::PathBuf, sync::Mutex};
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::{Update, UpdaterExt};
use tauri_plugin_clipboard_manager::ClipboardExt;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Tool { id:String, name:String, #[serde(default)] description:String, kind:String, url:Option<String>, path:Option<String> }
struct AppState { allowed:Mutex<HashSet<String>>, pending:Mutex<Option<Update>>, install:Mutex<Option<(Update,Vec<u8>)>> }
fn config_path(app:&tauri::AppHandle)->Result<PathBuf,String>{ app.path().app_config_dir().map(|p|p.join("tools.json")).map_err(|e|e.to_string()) }
fn read_tools(app:&tauri::AppHandle)->Result<Vec<Tool>,String>{let path=config_path(app)?;if !path.exists(){return Ok(vec![]);}let bytes=std::fs::read(path).map_err(|e|e.to_string())?;if bytes.len()>1_000_000{return Err("Shelf file is too large".into());}serde_json::from_slice(&bytes).map_err(|e|e.to_string())}
fn validate_url(value:&str)->Result<(),String>{let url=url::Url::parse(value).map_err(|_|"Enter a valid website URL")?;if !matches!(url.scheme(),"https"|"http")||url.host_str().is_none()||!url.username().is_empty()||url.password().is_some(){return Err("Only ordinary http and https website links are supported".into());}Ok(())}
#[tauri::command]
fn app_version(app:tauri::AppHandle)->String{app.package_info().version.to_string()}
#[tauri::command]
fn load_tools(app:tauri::AppHandle,state:State<AppState>)->Result<Vec<Tool>,String>{let tools=read_tools(&app)?;let mut allowed=state.allowed.lock().map_err(|e|e.to_string())?;for t in &tools{if let Some(path)=&t.path{allowed.insert(path.clone());}}Ok(tools)}
#[tauri::command]
fn save_tools(app:tauri::AppHandle,state:State<AppState>,tools:Vec<Tool>)->Result<(),String>{if tools.len()>200{return Err("The shelf supports up to 200 added tools".into());}let allowed=state.allowed.lock().map_err(|e|e.to_string())?;let mut ids=HashSet::new();for t in &tools{if t.name.trim().is_empty()||t.name.len()>240||t.description.len()>720||!ids.insert(&t.id)||["sign","pdf","image","key"].contains(&t.id.as_str()){return Err("Invalid tool details".into());}match t.kind.as_str(){"url"=>validate_url(t.url.as_deref().unwrap_or(""))?,"local"=>{if !t.path.as_ref().is_some_and(|p|allowed.contains(p)){return Err("Choose local apps using the file picker first".into());}},_=>return Err("Unsupported tool type".into())}}
 let path=config_path(&app)?;std::fs::create_dir_all(path.parent().unwrap()).map_err(|e|e.to_string())?;let tmp=path.with_extension("tmp");std::fs::write(&tmp,serde_json::to_vec_pretty(&tools).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;std::fs::rename(tmp,path).map_err(|e|e.to_string())}
#[derive(Serialize)] struct ChosenApp {name:String,path:String}
#[tauri::command]
async fn choose_app(app:tauri::AppHandle,state:State<'_,AppState>)->Result<Option<ChosenApp>,String>{let handle=app.clone();let selected=tauri::async_runtime::spawn_blocking(move||handle.dialog().file().set_title("Choose an app for your shelf").blocking_pick_file()).await.map_err(|e|e.to_string())?;let Some(selected)=selected else{return Ok(None)};let path=selected.into_path().map_err(|e|e.to_string())?;if !path.is_file(){return Err("Choose an app file".into());}let value=path.to_string_lossy().into_owned();state.allowed.lock().map_err(|e|e.to_string())?.insert(value.clone());Ok(Some(ChosenApp{name:path.file_stem().unwrap_or_default().to_string_lossy().into_owned(),path:value}))}
#[tauri::command]
fn open_url(app:tauri::AppHandle,url:String)->Result<(),String>{validate_url(&url)?;app.opener().open_url(url,None::<&str>).map_err(|e|e.to_string())}
#[tauri::command]
fn open_tool(app:tauri::AppHandle,id:String)->Result<(),String>{let t=read_tools(&app)?.into_iter().find(|t|t.id==id).ok_or("Tool not found")?;if t.kind=="url"{return open_url(app,t.url.ok_or("Missing URL")?);}let path=PathBuf::from(t.path.ok_or("Missing app path")?);if !path.is_file(){return Err("This app has moved. Remove it from the shelf and add it again.".into());}
 #[cfg(target_os="windows")]{if path.extension().is_some_and(|x|x.to_string_lossy().eq_ignore_ascii_case("exe")){std::process::Command::new(path).spawn().map_err(|e|e.to_string())?;}else{app.opener().open_path(path.to_string_lossy(),None::<&str>).map_err(|e|e.to_string())?;}}
 #[cfg(not(target_os="windows"))]{use std::os::unix::fs::PermissionsExt;if std::fs::metadata(&path).map_err(|e|e.to_string())?.permissions().mode()&0o111!=0 && path.extension().is_none_or(|e|e!="desktop"){std::process::Command::new(&path).spawn().map_err(|e|e.to_string())?;}else{app.opener().open_path(path.to_string_lossy(),None::<&str>).map_err(|e|e.to_string())?;}}Ok(())}
#[tauri::command]
async fn save_file(app:tauri::AppHandle,name:String,base64:String)->Result<bool,String>{if base64.len()>220_000_000{return Err("This export is too large. Try smaller files.".into());}let name=PathBuf::from(&name).file_name().ok_or("Invalid filename")?.to_string_lossy().into_owned();tauri::async_runtime::spawn_blocking(move||{let Some(file)=app.dialog().file().set_file_name(name).blocking_save_file() else{return Ok(false)};let path=file.into_path().map_err(|e|e.to_string())?;let bytes=base64::engine::general_purpose::STANDARD.decode(base64).map_err(|e|e.to_string())?;std::fs::write(path,bytes).map_err(|e|e.to_string())?;Ok(true)}).await.map_err(|e|e.to_string())?}
#[tauri::command]
fn copy_text(app:tauri::AppHandle,text:String)->Result<(),String>{if text.len()>1000{return Err("Text is too long".into());}app.clipboard().write_text(text).map_err(|e|e.to_string())}
#[derive(Serialize)] struct UpdateInfo {version:String}
#[tauri::command]
async fn check_update(app:tauri::AppHandle,state:State<'_,AppState>)->Result<Option<UpdateInfo>,String>{let update=app.updater_builder().timeout(std::time::Duration::from_secs(25)).build().map_err(|e|e.to_string())?.check().await.map_err(|e|e.to_string())?;let result=update.as_ref().map(|u|UpdateInfo{version:u.version.clone()});*state.pending.lock().map_err(|e|e.to_string())?=update;Ok(result)}
#[tauri::command]
async fn download_update(state:State<'_,AppState>)->Result<(),String>{let update=state.pending.lock().map_err(|e|e.to_string())?.take().ok_or("Check for updates first")?;let bytes=update.download(|_,_|{},||{}).await.map_err(|e|e.to_string())?;*state.install.lock().map_err(|e|e.to_string())?=Some((update,bytes));Ok(())}
#[tauri::command]
async fn install_update(app:tauri::AppHandle,state:State<'_,AppState>)->Result<(),String>{let (update,bytes)=state.install.lock().map_err(|e|e.to_string())?.take().ok_or("Download an update first")?;
 #[cfg(target_os="linux")]{let _=update;let exe=install_linux(&bytes)?;std::process::Command::new("/bin/sh").arg("-c").arg("sleep 1; exec \"$1\"").arg("mytools-restart").arg(exe).spawn().map_err(|e|e.to_string())?;app.exit(0);}
 #[cfg(not(target_os="linux"))]{update.install(bytes).map_err(|e|e.to_string())?;app.restart();}Ok(())}
#[cfg(target_os="linux")]
fn install_linux(bytes:&[u8])->Result<PathBuf,String>{use std::io::Write;use std::os::unix::fs::{OpenOptionsExt,PermissionsExt};let exe=std::env::current_exe().map_err(|e|e.to_string())?;let mut archive=tar::Archive::new(flate2::read::GzDecoder::new(bytes));let mut binary=vec![];for entry in archive.entries().map_err(|e|e.to_string())?{let mut entry=entry.map_err(|e|e.to_string())?;if entry.path().map_err(|e|e.to_string())?.as_ref()==std::path::Path::new("mytools") && entry.header().entry_type().is_file(){if entry.size()>200_000_000{return Err("Update is too large".into());}std::io::Read::read_to_end(&mut entry,&mut binary).map_err(|e|e.to_string())?;break;}}if !binary.starts_with(b"\x7fELF"){return Err("The update does not contain a Linux executable".into());}let next=exe.with_extension("update");let backup=exe.with_extension("previous");let mut file=std::fs::OpenOptions::new().write(true).create_new(true).mode(0o755).open(&next).map_err(|e|format!("Could not stage update: {e}. Install MyTools in a folder you own."))?;file.write_all(&binary).map_err(|e|e.to_string())?;file.sync_all().map_err(|e|e.to_string())?;std::fs::set_permissions(&next,std::fs::Permissions::from_mode(0o755)).map_err(|e|e.to_string())?;let _=std::fs::remove_file(&backup);std::fs::rename(&exe,&backup).map_err(|e|e.to_string())?;if let Err(e)=std::fs::rename(&next,&exe){let _=std::fs::rename(&backup,&exe);return Err(e.to_string());}Ok(exe)}
fn main(){tauri::Builder::default().plugin(tauri_plugin_single_instance::init(|app,_,_|{if let Some(window)=app.get_webview_window("main"){let _=window.show();let _=window.set_focus();}})).plugin(tauri_plugin_dialog::init()).plugin(tauri_plugin_opener::init()).plugin(tauri_plugin_clipboard_manager::init()).plugin(tauri_plugin_updater::Builder::new().build()).manage(AppState{allowed:Mutex::new(HashSet::new()),pending:Mutex::new(None),install:Mutex::new(None)}).invoke_handler(tauri::generate_handler![app_version,load_tools,save_tools,choose_app,open_url,open_tool,save_file,copy_text,check_update,download_update,install_update]).run(tauri::generate_context!()).expect("MyTools could not start");}
