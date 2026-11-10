#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde_json::json;
use tauri::{Emitter, Manager, WebviewUrl};
use tauri::webview::{DownloadEvent, NewWindowResponse, PageLoadEvent, WebviewBuilder};
use tauri::{LogicalPosition, LogicalSize};
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(windows)]
use webview2_com::{AcceleratorKeyPressedEventHandler, Microsoft::Web::WebView2::Win32::{COREWEBVIEW2_KEY_EVENT_KIND, COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN, ICoreWebView2AcceleratorKeyPressedEventArgs}};
#[cfg(windows)]
use windows::Win32::Foundation::{HWND, RECT};
#[cfg(windows)]
use windows::Win32::Graphics::Gdi::{CombineRgn, CreateRectRgn, DeleteObject, SetWindowRgn, RGN_DIFF};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetClientRect, GetWindow, GetWindowRect, IsWindowVisible, GW_CHILD, GW_HWNDNEXT};
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_L, VK_N, VK_R, VK_SHIFT, VK_T, VK_TAB, VK_W};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
static CLEARER_SEQUENCE: AtomicU64 = AtomicU64::new(0);

const AD_BLOCK_SCRIPT: &str = r#"(() => {
  const blocked = /(?:doubleclick|googlesyndication|googleadservices|adservice\.google|adnxs|taboola|outbrain|criteo|adroll|advertising\.com|adsystem|amazon-adsystem|adsrvr|pubmatic|rubiconproject|smartadserver|scorecardresearch|openx\.net|adform\.net)/i;
  const resourceSelector = 'iframe, img, script, video, source, link[rel="preload"]';
  const styleId = '__cherry_ad_block_css';
  window.__cherryAdBlockEnabled = __CHERRY_AD_BLOCK__;
  if (window.__cherryAdBlockInstalled) return;
  window.__cherryAdBlockInstalled = true;
  const shouldBlock = value => window.__cherryAdBlockEnabled && blocked.test(String(value || ''));
  const nativeFetch = window.fetch?.bind(window);
  if (nativeFetch) window.fetch = (input, init) => {
    const url = typeof input === 'string' ? input : input?.url;
    return shouldBlock(url) ? Promise.resolve(new Response(null, { status: 204 })) : nativeFetch(input, init);
  };
  const nativeOpen = XMLHttpRequest.prototype.open;
  const nativeSend = XMLHttpRequest.prototype.send;
  XMLHttpRequest.prototype.open = function(method, url, ...rest) { this.__cherryBlocked = shouldBlock(url); return nativeOpen.call(this, method, url, ...rest); };
  XMLHttpRequest.prototype.send = function(...args) { if (this.__cherryBlocked) { try { this.abort(); } catch {} return; } return nativeSend.apply(this, args); };
  const scan = root => {
    if (!window.__cherryAdBlockEnabled) return;
    const elements = [];
    if (root.matches?.(resourceSelector)) elements.push(root);
    root.querySelectorAll?.(resourceSelector).forEach(element => elements.push(element));
    for (const element of elements) if (shouldBlock(element.getAttribute('src'))) element.remove();
  };
  const syncBlocker = () => {
    const root = document.documentElement;
    if (!root) return;
    const style = document.getElementById(styleId);
    if (window.__cherryAdBlockEnabled) {
      if (!style) {
        const element = document.createElement('style');
        element.id = styleId;
        element.textContent = '[id*=ad-container i],[class*=ad-container i],[id^=google_ads i],[class^=google-ad i],[data-ad],[data-ad-slot],.adsbygoogle,.ad-banner,.advertisement{display:none!important}';
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

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let window = tauri::Window::builder(app, "main")
                .title("Cherry Browser")
                .inner_size(1280.0, 850.0)
                .min_inner_size(900.0, 620.0)
                .build()?;
            let shell = WebviewBuilder::new("shell", WebviewUrl::App("index.html".into())).auto_resize();
            window.add_child(shell, LogicalPosition::new(0.0, 0.0), LogicalSize::new(1280.0, 850.0))?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![create_page, page_action, set_page_bounds, set_page_visible, focus_page, set_page_cutout, close_page, clear_page_data, app_version])
        .run(tauri::generate_context!())
        .expect("falha ao iniciar o Cherry Browser");
}

