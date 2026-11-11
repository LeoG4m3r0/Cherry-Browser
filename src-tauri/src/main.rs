#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde_json::json;
use sha2::{Digest, Sha256};
use tauri::{Emitter, Manager, WebviewUrl};
use tauri::webview::{DownloadEvent, NewWindowResponse, PageLoadEvent, WebviewBuilder};
use tauri::{LogicalPosition, LogicalSize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::collections::HashMap;
use std::process::Command;
use std::io::{Read, Write};
#[cfg(windows)]
use webview2_com::{AcceleratorKeyPressedEventHandler, WebResourceRequestedEventHandler, Microsoft::Web::WebView2::Win32::{COREWEBVIEW2_KEY_EVENT_KIND, COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN, COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL, ICoreWebView2AcceleratorKeyPressedEventArgs}};
#[cfg(windows)]
use windows::Win32::Foundation::{HWND, RECT};
#[cfg(windows)]
use windows::core::{HSTRING, PWSTR};
#[cfg(windows)]
use windows::Win32::Graphics::Gdi::{CombineRgn, CreateRectRgn, DeleteObject, SetWindowRgn, RGN_DIFF};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetClientRect, GetWindow, GetWindowRect, IsWindowVisible, SetWindowPos, HWND_TOP, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, GW_CHILD, GW_HWNDNEXT};
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_L, VK_N, VK_R, VK_SHIFT, VK_T, VK_TAB, VK_W};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
static CLEARER_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static AD_BLOCK_STATES: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
const DNS_SETTINGS_FILE: &str = "browser-settings.json";

fn dns_settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path().app_data_dir().map(|dir| dir.join(DNS_SETTINGS_FILE)).map_err(|e| e.to_string())
}

fn read_dns_provider(path: &std::path::Path) -> String {
    std::fs::read(path).ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|settings| settings.get("dnsProvider").and_then(|value| value.as_str()).map(str::to_owned))
        .filter(|provider| matches!(provider.as_str(), "cloudflare" | "google" | "quad9" | "system"))
        .unwrap_or_else(|| "cloudflare".into())
}

fn apply_dns_provider(provider: &str) {
    let dns_flags = match provider {
        "google" => "--dns-over-https-mode=secure --dns-over-https-templates=https://dns.google/dns-query{?dns}",
        "quad9" => "--dns-over-https-mode=secure --dns-over-https-templates=https://dns.quad9.net/dns-query{?dns}",
        "system" => "--dns-over-https-mode=off",
        _ => "--dns-over-https-mode=secure --dns-over-https-templates=https://cloudflare-dns.com/dns-query{?dns}",
    };
    let inherited = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").unwrap_or_default();
    let arguments = if inherited.trim().is_empty() { dns_flags.to_owned() }
        else { format!("{} {}", inherited.trim(), dns_flags) };
    std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", arguments);
}

#[tauri::command]
fn get_dns_provider(app: tauri::AppHandle) -> Result<String, String> {
    Ok(read_dns_provider(&dns_settings_path(&app)?))
}

#[tauri::command]
fn set_dns_provider(app: tauri::AppHandle, provider: String) -> Result<bool, String> {
    if !matches!(provider.as_str(), "cloudflare" | "google" | "quad9" | "system") {
        return Err("Provedor DNS inválido".into());
    }
    let path = dns_settings_path(&app)?;
    let changed = read_dns_provider(&path) != provider;
    let parent = path.parent().ok_or_else(|| "Pasta de configurações inválida".to_string())?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let contents = serde_json::to_vec_pretty(&json!({ "dnsProvider": provider })).map_err(|e| e.to_string())?;
    std::fs::write(path, contents).map_err(|e| e.to_string())?;
    Ok(changed)
}

#[tauri::command]
fn restart_app(app: tauri::AppHandle) { app.restart(); }

#[cfg(windows)]
fn raise_spotify_webview(view: &tauri::Webview) -> Result<(), String> {
    view.with_webview(|platform| unsafe {
        let mut hwnd = HWND::default();
        if platform.controller().ParentWindow(&mut hwnd).is_ok() && !hwnd.0.is_null() {
            let _ = SetWindowPos(hwnd, Some(HWND_TOP), 0, 0, 0, 0, SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE);
        }
    }).map_err(|e| e.to_string())
}

#[cfg(not(windows))]
fn raise_spotify_webview(_view: &tauri::Webview) -> Result<(), String> { Ok(()) }

const AD_BLOCK_SCRIPT: &str = r#"(() => {
  const resourceSelector = 'iframe, img, script, video, source, link[rel="preload"], link[rel="stylesheet"]';
  const styleId = '__cherry_ad_block_css';
  window.__cherryAdBlockEnabled = __CHERRY_AD_BLOCK__;
  if (window.__cherryAdBlockInstalled) return;
  window.__cherryAdBlockInstalled = true;
  const shouldBlock = () => window.__cherryAdBlockEnabled;
  const scan = root => {
    if (!window.__cherryAdBlockEnabled) return;
    const elements = [];
    if (root.matches?.(resourceSelector)) elements.push(root);
    root.querySelectorAll?.(resourceSelector).forEach(element => elements.push(element));
    for (const element of elements) if (shouldBlock() && /(?:doubleclick|googlesyndication|googleadservices|adservice|adnxs|taboola|outbrain|criteo|adroll|adsystem|amazon-adsystem|adsrvr|pubmatic|rubiconproject|smartadserver|scorecardresearch|openx|adform)/i.test(element.getAttribute('src') || element.getAttribute('href') || '')) element.remove();
  };
  const syncBlocker = () => {
    const root = document.documentElement;
    if (!root) return;
    const style = document.getElementById(styleId);
    if (window.__cherryAdBlockEnabled) {
      if (!style) {
        const element = document.createElement('style');
        element.id = styleId;
        element.textContent = '[id*=ad-container i],[class*=ad-container i],[id^=google_ads i],[class^=google-ad i],[data-ad],[data-ad-slot],[data-advertisement],.adsbygoogle,.ad-banner,.advertisement,.ad-unit,.ad-wrapper,.sponsored-content,.sponsored-post,.sponsor-banner,.promo-ad,.taboola,.outbrain,.trc_rbox,.mgid{display:none!important}';
        root.appendChild(element);
      }
      scan(root);
    } else {
      style?.remove();
    }
  };
  window.__cherrySetAdBlockEnabled = enabled => {
    window.__cherryAdBlockEnabled = Boolean(enabled);
    syncBlocker();
  };
  let scanQueued = false;
  let pendingRoots = new Set();
  new MutationObserver(records => {
    if (!window.__cherryAdBlockEnabled) return;
    for (const record of records) for (const node of record.addedNodes) if (node.nodeType === 1) pendingRoots.add(node);
    if (scanQueued || !pendingRoots.size) return;
    scanQueued = true;
    queueMicrotask(() => {
      scanQueued = false;
      const roots = [...pendingRoots];
      pendingRoots = new Set();
      for (const root of roots) {
        if (!root.isConnected || roots.some(other => other !== root && other.contains(root))) continue;
        scan(root);
      }
    });
  }).observe(document, { childList: true, subtree: true });
  if (document.documentElement) syncBlocker();
  else document.addEventListener('DOMContentLoaded', syncBlocker, { once: true });
})();"#;

// Native WebView2 filtering prevents known ad and tracking requests from being downloaded.
const AD_HOSTS: &[&str] = &[
    "doubleclick.net", "googlesyndication.com", "googleadservices.com", "adservice.google.com",
    "adnxs.com", "adsrvr.org", "adform.net", "adroll.com", "advertising.com",
    "amazon-adsystem.com", "bidswitch.net", "bluekai.com", "casalemedia.com", "contextweb.com",
    "criteo.com", "criteo.net", "demdex.net", "eyeota.net",
    "hotjar.com", "imasdk.googleapis.com", "imrworldwide.com", "improvedigital.com",
    "liadm.com", "lijit.com", "mathtag.com", "media.net", "moatads.com", "openx.net",
    "outbrain.com", "pippio.com", "pubmatic.com", "quantserve.com", "scorecardresearch.com",
    "serving-sys.com", "sharethrough.com", "simpli.fi", "smartadserver.com", "taboola.com",
    "tapad.com", "teads.tv", "thetradedesk.com", "tidaltv.com", "tremorhub.com",
    "triplelift.com", "yieldmo.com", "zedo.com", "zemanta.com", "doubleverify.com",
    "ad.gt", "adnxs-simple.com", "adsafeprotected.com", "adsymptotic.com", "agkn.com",
    "appnexus.com", "bannersnack.com", "betrad.com", "chartbeat.com", "clicktale.net",
    "dotomi.com", "exelator.com", "flashtalking.com", "gumgum.com", "innovid.com",
    "krxd.net", "nexac.com", "parsely.com", "rlcdn.com", "sascdn.com", "spotxchange.com",
    "turn.com", "undertone.com", "verizonmedia.com"
];

fn is_ad_host(host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    AD_HOSTS.iter().any(|entry| {
        let domain = *entry;
        host == domain || host.strip_suffix(domain).map_or(false, |prefix| prefix.ends_with('.'))
    })
}

#[cfg(windows)]
fn install_ad_blocker(view: &tauri::Webview, label: &str, enabled: bool) -> Result<(), String> {
    let state = Arc::new(AtomicBool::new(enabled));
    AD_BLOCK_STATES.get_or_init(|| Mutex::new(HashMap::new()))
        .lock().map_err(|_| "Estado do bloqueador indisponível".to_string())?
        .insert(label.to_owned(), state.clone());
    let install_result = Arc::new(Mutex::new(None));
    let result_slot = install_result.clone();
    view.with_webview(move |platform| {
        let result = unsafe { (|| -> Result<(), String> {
            let controller = platform.controller();
            let core = controller.CoreWebView2().map_err(|e| e.to_string())?;
            let env = platform.environment();
            let filter = HSTRING::from("*");
            core.AddWebResourceRequestedFilter(&filter, COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL)
                .map_err(|e| e.to_string())?;
            let handler = WebResourceRequestedEventHandler::create(Box::new(move |_sender, args| {
                let Some(args) = args else { return Ok(()); };
                if !state.load(Ordering::Relaxed) { return Ok(()); }
                let Ok(request) = args.Request() else { return Ok(()); };
                let mut uri = PWSTR::null();
                if request.Uri(&mut uri).is_err() { return Ok(()); }
                let Ok(uri) = uri.to_string() else { return Ok(()); };
                let blocked = tauri::Url::parse(&uri).ok()
                    .and_then(|url| url.host_str().map(is_ad_host)).unwrap_or(false);
                if blocked {
                    let status = HSTRING::from("No Content");
                    let headers = HSTRING::from("Cache-Control: no-store\r\n");
                    if let Ok(response) = env.CreateWebResourceResponse(None, 204, &status, &headers) {
                        let _ = args.SetResponse(&response);
                    }
                }
                Ok(())
            }));
            let mut token = 0i64;
            core.add_WebResourceRequested(&handler, &mut token).map_err(|e| e.to_string())
        })() };
        if let Ok(mut slot) = result_slot.lock() { *slot = Some(result); }
    }).map_err(|e| e.to_string())?;
    let configured = install_result.lock().map_err(|_| "Estado do bloqueador indisponível".to_string())?
        .take().ok_or_else(|| "Não foi possível configurar o bloqueador nativo".to_string())?;
    configured
}

#[cfg(not(windows))]
fn install_ad_blocker(_view: &tauri::Webview, _label: &str, _enabled: bool) -> Result<(), String> { Ok(()) }

fn emit_shell<R: tauri::Runtime, P: serde::Serialize + Clone>(app: &tauri::AppHandle<R>, event: &str, payload: P) {
    let _ = app.emit_to("shell", event, payload);
}

#[tauri::command]
async fn create_page(
    app: tauri::AppHandle,
    label: String,
    url: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    incognito: bool,
    ad_block: bool,
    visible: bool,
) -> Result<(), String> {
    if !label.starts_with("tab_") || !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("Identificador de aba inválido".into());
    }
    let target = url.parse::<tauri::Url>().map_err(|e| format!("Endereço inválido: {e}"))?;
    if target.scheme() != "http" && target.scheme() != "https" {
        return Err("Cherry Browser abre apenas endereços HTTP e HTTPS".into());
    }

    let window = app.get_window("main").ok_or_else(|| "Janela principal não encontrada".to_string())?;
    let app_navigation = app.clone();
    let app_title = app.clone();
    let app_load = app.clone();
    let app_download = app.clone();
    let app_popup = app.clone();
    let label_navigation = label.clone();
    let label_title = label.clone();
    let label_load = label.clone();
    let label_download = label.clone();
    let label_popup = label.clone();

    let ad_script = AD_BLOCK_SCRIPT.replace("__CHERRY_AD_BLOCK__", if ad_block { "true" } else { "false" });
    let builder = WebviewBuilder::new(label.clone(), WebviewUrl::External(target))
        .incognito(incognito)
        .initialization_script(ad_script)
        .on_navigation(move |next| {
            emit_shell(&app_navigation, "cherry:navigation", json!({
                "label": label_navigation,
                "url": next.as_str()
            }));
            next.scheme() == "http" || next.scheme() == "https"
        })
        .on_document_title_changed(move |_view, title| {
            emit_shell(&app_title, "cherry:title", json!({
                "label": label_title,
                "title": title
            }));
        })
        .on_page_load(move |_view, payload| {
            let event = match payload.event() {
                PageLoadEvent::Started => "started",
                PageLoadEvent::Finished => "finished",
            };
            emit_shell(&app_load, "cherry:page-load", json!({
                "label": label_load,
                "url": payload.url().as_str(),
                "state": event
            }));
        })
        .on_download(move |_view, event| {
            match event {
                DownloadEvent::Requested { url, destination } => {
                    if destination.as_os_str().is_empty() {
                        let base = app_download.path().download_dir()
                            .or_else(|_| app_download.path().app_data_dir())
                            .unwrap_or_else(|_| std::env::temp_dir());
                        let _ = std::fs::create_dir_all(&base);
                        let suggested = url.path_segments()
                            .and_then(|mut parts| parts.next_back())
                            .filter(|name| !name.is_empty())
                            .unwrap_or("download");
                        *destination = base.join(suggested);
                    }
                    emit_shell(&app_download, "cherry:download", json!({
                        "label": label_download,
                        "url": url.as_str(),
                        "path": destination.to_string_lossy(),
                        "state": "started",
                        "incognito": incognito
                    }));
                }
                DownloadEvent::Finished { url, path, success } => {
                    emit_shell(&app_download, "cherry:download", json!({
                        "label": label_download,
                        "url": url.as_str(),
                        "path": path.map(|p| p.to_string_lossy().to_string()),
                        "state": if success { "finished" } else { "failed" },
                        "incognito": incognito
                    }));
                }
                _ => {}
            }
            true
        })
        .on_new_window(move |new_url, _features| {
            emit_shell(&app_popup, "cherry:new-window", json!({
                "label": label_popup,
                "url": new_url.as_str()
            }));
            NewWindowResponse::Deny
        });

    let view = window
        .add_child(builder, LogicalPosition::new(x, y), LogicalSize::new(width, height))
        .map_err(|e| e.to_string())?;
    install_ad_blocker(&view, &label, ad_block)?;
    #[cfg(windows)]
    install_shortcuts(&view, app.clone());
    if !visible {
        view.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(windows)]
fn install_shortcuts(view: &tauri::Webview, app: tauri::AppHandle) {
    let _ = view.with_webview(move |platform| {
        let controller = platform.controller();
        let handler = AcceleratorKeyPressedEventHandler::create(Box::new(move |_controller, args: Option<ICoreWebView2AcceleratorKeyPressedEventArgs>| {
            if let Some(args) = args {
                let mut event_kind = COREWEBVIEW2_KEY_EVENT_KIND::default();
                let mut virtual_key = 0u32;
                let event = unsafe { args.KeyEventKind(&mut event_kind).and_then(|_| args.VirtualKey(&mut virtual_key)) };
                if event.is_ok() && event_kind == COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN
                    && unsafe { GetKeyState(VK_CONTROL.0.into()) } < 0 {
                    let shift = unsafe { GetKeyState(VK_SHIFT.0.into()) } < 0;
                    let action = match virtual_key {
                        key if key == VK_T.0 as u32 && shift => Some(("restore-tab", false)),
                        key if key == VK_T.0 as u32 => Some(("new-tab", false)),
                        key if key == VK_L.0 as u32 => Some(("focus-address", false)),
                        key if key == VK_W.0 as u32 => Some(("close-tab", false)),
                        key if key == VK_R.0 as u32 => Some(("reload", false)),
                        key if key == VK_TAB.0 as u32 => Some(("next-tab", shift)),
                        key if key == VK_N.0 as u32 && shift => Some(("private-tab", false)),
                        _ => None,
                    };
                    if let Some((action, reverse)) = action {
                        let _ = unsafe { args.SetHandled(true) };
                        let _ = app.emit_to("shell", "cherry:shortcut", json!({ "action": action, "reverse": reverse }));
                    }
                }
            }
            Ok(())
        }));
        let mut token = 0i64;
        let _ = unsafe { controller.add_AcceleratorKeyPressed(&handler, &mut token) };
    });
}

#[tauri::command]
fn page_action(
    app: tauri::AppHandle,
    label: String,
    action: String,
    url: Option<String>,
) -> Result<(), String> {
    let view = app.get_webview(&label).ok_or_else(|| "Aba não encontrada".to_string())?;
    match action.as_str() {
        "go" => {
            let target = url.ok_or_else(|| "Endereço vazio".to_string())?;
            let parsed = target.parse::<tauri::Url>().map_err(|e| e.to_string())?;
            if parsed.scheme() != "http" && parsed.scheme() != "https" {
                return Err("Cherry Browser abre apenas endereços HTTP e HTTPS".into());
            }
            let encoded = serde_json::to_string(parsed.as_str()).map_err(|e| e.to_string())?;
            view.eval(&format!("location.href = {encoded}" )).map_err(|e| e.to_string())
        }
        "back" => view.eval("history.back()").map_err(|e| e.to_string()),
        "forward" => view.eval("history.forward()").map_err(|e| e.to_string()),
        "reload" => view.eval("location.reload()").map_err(|e| e.to_string()),
        "stop" => view.eval("window.stop()").map_err(|e| e.to_string()),
        "adblock" => {
            let enabled = url.as_deref() == Some("on");
            if let Some(states) = AD_BLOCK_STATES.get() {
                if let Ok(states) = states.lock() {
                    if let Some(state) = states.get(&label) { state.store(enabled, Ordering::Relaxed); }
                }
            }
            view.eval(&format!("window.__cherrySetAdBlockEnabled?.({enabled})" )).map_err(|e| e.to_string())
        }
        _ => Err("Ação desconhecida".into()),
    }
}

#[tauri::command]
fn set_page_bounds(app: tauri::AppHandle, label: String, x: f64, y: f64, width: f64, height: f64) -> Result<(), String> {
    let view = app.get_webview(&label).ok_or_else(|| "Aba não encontrada".to_string())?;
    view.set_position(LogicalPosition::new(x, y)).map_err(|e| e.to_string())?;
    view.set_size(LogicalSize::new(width, height)).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_spotify_embed(app: tauri::AppHandle, url: String, x: f64, y: f64, width: f64, height: f64) -> Result<(), String> {
    let target = url.parse::<tauri::Url>().map_err(|e| e.to_string())?;
    if target.scheme() != "https" || target.host_str() != Some("open.spotify.com") || !target.path().starts_with("/embed/") {
        return Err("O player só pode abrir embeds oficiais do Spotify".into());
    }
    let position = LogicalPosition::new(x, y);
    let size = LogicalSize::new(width.max(1.0), height.max(1.0));
    if let Some(view) = app.get_webview("spotify_embed") {
        view.set_position(position).map_err(|e| e.to_string())?;
        view.set_size(size).map_err(|e| e.to_string())?;
        let current = view.url().map(|current| current.to_string()).unwrap_or_default();
        if current != target.as_str() { view.navigate(target).map_err(|e| e.to_string())?; }
        view.show().map_err(|e| e.to_string())?;
        view.set_focus().map_err(|e| e.to_string())?;
        raise_spotify_webview(&view)?;
        return Ok(());
    }
    let window = app.get_window("main").ok_or_else(|| "Janela principal não encontrada".to_string())?;
    let app_load = app.clone();
    let builder = WebviewBuilder::new("spotify_embed", WebviewUrl::External(target))
        .incognito(false)
        .on_page_load(move |_view, payload| {
            let state = match payload.event() {
                PageLoadEvent::Started => "started",
                PageLoadEvent::Finished => "finished",
            };
            emit_shell(&app_load, "cherry:spotify-load", json!({ "url": payload.url().as_str(), "state": state }));
        })
        .on_navigation(|next| {
            let host = next.host_str().unwrap_or_default().to_ascii_lowercase();
            next.scheme() == "https" && (host == "open.spotify.com" || host == "spotify.com" || host.ends_with(".spotify.com"))
        });
    let view = window.add_child(builder, position, size).map_err(|e| e.to_string())?;
    view.show().map_err(|e| e.to_string())?;
    view.set_focus().map_err(|e| e.to_string())?;
    raise_spotify_webview(&view)
}

#[tauri::command]
fn set_spotify_embed_visible(app: tauri::AppHandle, visible: bool) -> Result<(), String> {
    if let Some(view) = app.get_webview("spotify_embed") {
        if visible { view.show() } else { view.hide() }.map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn set_page_visible(app: tauri::AppHandle, label: String, visible: bool) -> Result<(), String> {
    let view = app.get_webview(&label).ok_or_else(|| "Aba não encontrada".to_string())?;
    if visible { view.show() } else { view.hide() }.map_err(|e| e.to_string())
}

#[tauri::command]
fn focus_page(app: tauri::AppHandle, label: String) -> Result<(), String> {
    let view = app.get_webview(&label).ok_or_else(|| "Aba não encontrada".to_string())?;
    view.set_focus().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_page_cutout(app: tauri::AppHandle, x: f64, y: f64, width: f64, height: f64, enabled: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        let parent = app.get_window("main").ok_or_else(|| "Janela principal não encontrada".to_string())?;
        let parent_hwnd = parent.hwnd().map_err(|e| e.to_string())?;
        let mut children: Vec<(HWND, RECT, bool)> = Vec::new();
        unsafe {
            let mut child = GetWindow(parent_hwnd, GW_CHILD).unwrap_or_default();
            while !child.0.is_null() {
                let mut class_name = [0u16; 64];
                let class_len = GetClassNameW(child, &mut class_name);
                if class_len > 0 && String::from_utf16_lossy(&class_name[..class_len as usize]) == "WRY_WEBVIEW" {
                    let mut rect = RECT::default();
                    if GetWindowRect(child, &mut rect).is_ok() {
                        children.push((child, rect, IsWindowVisible(child).as_bool()));
                    }
                }
                child = GetWindow(child, GW_HWNDNEXT).unwrap_or_default();
            }
            let shell_top = children.iter().map(|(_, rect, _)| rect.top).min()
                .ok_or_else(|| "A interface do Cherry não foi encontrada".to_string())?;
            let target = children.iter().find(|(_, rect, visible)| *visible && rect.top > shell_top)
                .map(|(hwnd, _, _)| *hwnd)
                .ok_or_else(|| "A aba visível não foi encontrada".to_string())?;
            if !enabled {
                if SetWindowRgn(target, None, true) == 0 { return Err("Não foi possível restaurar a página".into()); }
            } else {
                let scale = parent.scale_factor().map_err(|e| e.to_string())?;
                let mut client = RECT::default();
                GetClientRect(target, &mut client).map_err(|e| e.to_string())?;
                let right_edge = client.right - client.left;
                let bottom_edge = client.bottom - client.top;
                let left = ((x * scale).round() as i32).clamp(0, right_edge);
                let top = ((y * scale).round() as i32).clamp(0, bottom_edge);
                let right = (((x + width) * scale).round() as i32).clamp(left, right_edge);
                let bottom = (((y + height) * scale).round() as i32).clamp(top, bottom_edge);
                let full = CreateRectRgn(0, 0, right_edge, bottom_edge);
                let hole = CreateRectRgn(left, top, right, bottom);
                if full.0.is_null() || hole.0.is_null() {
                    if !full.0.is_null() { let _ = DeleteObject(full.into()); }
                    if !hole.0.is_null() { let _ = DeleteObject(hole.into()); }
                    return Err("Não foi possível criar a área do popup".into());
                }
                let combined = CombineRgn(Some(full), Some(full), Some(hole), RGN_DIFF);
                let _ = DeleteObject(hole.into());
                if combined.0 == 0 || SetWindowRgn(target, Some(full), true) == 0 {
                    let _ = DeleteObject(full.into());
                    return Err("Não foi possível sobrepor o menu à página".into());
                }
            }
        }
    }
    #[cfg(not(windows))]
    let _ = (app, x, y, width, height, enabled);
    Ok(())
}

#[tauri::command]
fn close_page(app: tauri::AppHandle, label: String) -> Result<(), String> {
    if let Some(states) = AD_BLOCK_STATES.get() {
        if let Ok(mut states) = states.lock() { states.remove(&label); }
    }
    if let Some(view) = app.get_webview(&label) {
        view.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn clear_page_data(app: tauri::AppHandle, labels: Vec<String>) -> Result<(), String> {
    for label in labels {
        if let Some(view) = app.get_webview(&label) {
            view.clear_all_browsing_data().map_err(|e| e.to_string())?;
        }
    }
    let window = app.get_window("main").ok_or_else(|| "Janela principal não encontrada".to_string())?;
    let seq = CLEARER_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let label = format!("clear_data_{seq}");
    let blank = "about:blank".parse::<tauri::Url>().map_err(|e| e.to_string())?;
    let cleaner = window.add_child(
        WebviewBuilder::new(label, WebviewUrl::External(blank)).incognito(false),
        LogicalPosition::new(0.0, 0.0),
        LogicalSize::new(1.0, 1.0),
    ).map_err(|e| e.to_string())?;
    let result = cleaner.clear_all_browsing_data().map_err(|e| e.to_string());
    let close_result = cleaner.close().map_err(|e| e.to_string());
    result?;
    close_result?;
    Ok(())
}

#[tauri::command]
fn app_version() -> &'static str {
    APP_VERSION
}

fn emit_update_status(app: &tauri::AppHandle, state: &str, message: &str) {
    let _ = app.emit_to("shell", "cherry:update-status", json!({ "state": state, "message": message }));
}

#[tauri::command]
fn download_update(app: tauri::AppHandle, url: String, sha256: String, version: String) -> Result<(), String> {
    let target = url.parse::<tauri::Url>().map_err(|e| format!("URL de atualização inválida: {e}"))?;
    let file_name = target.path_segments().and_then(|mut segments| segments.next_back()).unwrap_or_default();
    if target.scheme() != "https" || target.host_str() != Some("github.com")
        || !target.path().contains("/releases/download/") || file_name != "Cherry-Browser-Setup.exe" {
        return Err("O instalador precisa vir de um release HTTPS oficial do Cherry Browser.".into());
    }
    let expected_hash = sha256.strip_prefix("sha256:").unwrap_or(&sha256).to_ascii_lowercase();
    if expected_hash.len() != 64 || !expected_hash.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err("O GitHub não forneceu um SHA-256 válido para verificar o instalador.".into());
    }
    if version.is_empty() || !version.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-') {
        return Err("Versão de atualização inválida.".into());
    }
    if app.get_webview("update_download").is_some() {
        return Err("Já existe uma atualização em andamento.".into());
    }
    let update_dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("updates");
    std::fs::create_dir_all(&update_dir).map_err(|e| e.to_string())?;
    let installer_path = update_dir.join(format!("Cherry-Browser-Setup-{version}.exe"));
    if installer_path.exists() { std::fs::remove_file(&installer_path).map_err(|e| e.to_string())?; }
    let expected_hash = expected_hash.clone();
    let installed_version = version.clone();
    let requested_url = target.to_string();
    let worker_app = app.clone();
    std::thread::spawn(move || {
        let result = (|| -> Result<(), String> {
            emit_update_status(&worker_app, "downloading", &format!("Baixando a versão {installed_version}…"));
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(600))
                .user_agent(concat!("Cherry-Browser/", env!("CARGO_PKG_VERSION")))
                .build().map_err(|e| format!("Não foi possível iniciar o download: {e}"))?;
            let mut response = client.get(requested_url).send()
                .and_then(reqwest::blocking::Response::error_for_status)
                .map_err(|e| format!("Falha ao baixar o instalador do GitHub: {e}"))?;
            let final_url = response.url();
            let final_host = final_url.host_str().unwrap_or_default();
            if final_url.scheme() != "https" || !matches!(final_host, "github.com" | "release-assets.githubusercontent.com" | "objects.githubusercontent.com") {
                return Err("O GitHub redirecionou o download para uma origem não permitida.".into());
            }
            if response.content_length().is_some_and(|length| length > 100 * 1024 * 1024) {
                return Err("O instalador excede o tamanho máximo permitido de 100 MB.".into());
            }
            let mut output = std::fs::File::create(&installer_path).map_err(|e| format!("Não foi possível criar o arquivo de atualização: {e}"))?;
            let mut hasher = Sha256::new();
            let mut total = 0u64;
            let mut buffer = [0u8; 64 * 1024];
            loop {
                let count = response.read(&mut buffer).map_err(|e| format!("O download foi interrompido: {e}"))?;
                if count == 0 { break; }
                total += count as u64;
                if total > 100 * 1024 * 1024 { return Err("O instalador excede o tamanho máximo permitido de 100 MB.".into()); }
                output.write_all(&buffer[..count]).map_err(|e| format!("Não foi possível salvar o instalador: {e}"))?;
                hasher.update(&buffer[..count]);
            }
            output.sync_all().map_err(|e| format!("Não foi possível finalizar o arquivo baixado: {e}"))?;
            drop(output);
            emit_update_status(&worker_app, "verifying", "Download concluído. Conferindo a integridade do instalador…");
            let actual_hash = format!("{:x}", hasher.finalize());
            if actual_hash != expected_hash { return Err("A verificação SHA-256 falhou; o instalador foi descartado.".into()); }
            emit_update_status(&worker_app, "installing", &format!("Instalando a versão {installed_version}. O Cherry será reaberto ao concluir…"));
            Command::new(&installer_path).arg("--apply-update").arg(std::process::id().to_string()).spawn()
                .map_err(|e| format!("O download foi verificado, mas o instalador não iniciou: {e}"))?;
            worker_app.exit(0);
            Ok(())
        })();
        if let Err(error) = result {
            let _ = std::fs::remove_file(&installer_path);
            emit_update_status(&worker_app, "failed", &error);
        }
    });
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let dns_provider = dns_settings_path(app.handle())
                .map(|settings_path| read_dns_provider(&settings_path))
                .unwrap_or_else(|_| "cloudflare".into());
            apply_dns_provider(&dns_provider);
            let window = tauri::Window::builder(app, "main")
                .title("Cherry Browser")
                .inner_size(1280.0, 850.0)
                .min_inner_size(900.0, 620.0)
                .build()?;
            let shell = WebviewBuilder::new("shell", WebviewUrl::App("index.html".into())).auto_resize();
            window.add_child(shell, LogicalPosition::new(0.0, 0.0), LogicalSize::new(1280.0, 850.0))?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![create_page, page_action, set_page_bounds, set_page_visible, focus_page, set_page_cutout, open_spotify_embed, set_spotify_embed_visible, close_page, clear_page_data, app_version, download_update, get_dns_provider, set_dns_provider, restart_app])
        .run(tauri::generate_context!())
        .expect("falha ao iniciar o Cherry Browser");
}

