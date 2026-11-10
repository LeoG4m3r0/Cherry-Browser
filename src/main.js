import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
const $ = selector => document.querySelector(selector);
const read = (key, fallback = []) => { try { return JSON.parse(localStorage.getItem(key)) ?? fallback; } catch { return fallback; } };
const save = (key, value) => localStorage.setItem(key, JSON.stringify(value));
const readList = key => { const value = read(key); return Array.isArray(value) ? value : []; };
const state = { tabs: [], active: null, seq: 0, private: false, version: '0.4.1', panel: null, menuCutout: false };
const engines = {
  google: query => `https://www.google.com/search?q=${query}`,
  bing: query => `https://www.bing.com/search?q=${query}`,
  brave: query => `https://search.brave.com/search?q=${query}`,
  duckduckgo: query => `https://duckduckgo.com/?q=${query}`
};
const adBlockEnabled = () => localStorage.getItem('cherry.adBlock') !== 'false';
function searchUrl(text) {
  const value = text.trim();
  if (!value) return 'https://www.google.com';
  if (/^(https?|about):/i.test(value)) return value;
  if (/^(localhost|127\.0\.0\.1)(:\d+)?([/].*)?$/i.test(value) || /^[\w-]+(\.[\w-]+)+(:\d+)?([/].*)?$/i.test(value)) return 'https://' + value;
  const engine = localStorage.getItem('cherry.engine') || 'google';
  return (engines[engine] || engines.google)(encodeURIComponent(value));
}
let toastTimer;
function toast(message) { const element = $('#toast'); element.textContent = message; element.classList.add('show'); clearTimeout(toastTimer); toastTimer = setTimeout(() => element.classList.remove('show'), 2300); }
function active() { return state.tabs.find(tab => tab.id === state.active); }
function renderTabs(scrollToEnd = false) {
  const container = $('#tabs'), newTabButton = $('#newTab'), scrollLeft = container.scrollLeft;
  container.replaceChildren();
  for (const tab of state.tabs) {
    const button = document.createElement('button'); button.className = 'tab' + (tab.id === state.active ? ' active' : '');
    button.innerHTML = `<span class="tab-dot">${tab.private ? '◉' : '●'}</span><span class="tab-title"></span><span class="tab-close">×</span>`;
    button.querySelector('.tab-title').textContent = tab.title || 'Nova aba';
    button.onclick = event => event.target.closest('.tab-close') ? closeTab(tab.id) : activate(tab.id);
    container.append(button);
  }
  container.append(newTabButton);
  container.scrollLeft = scrollToEnd ? container.scrollWidth : scrollLeft;
}
function renderBookmarks() {
  const container = $('#bookmarkBar'), items = readList('cherry.bookmarks'); container.replaceChildren();
  for (const item of items.slice(0, 8)) { const button = document.createElement('button'); button.textContent = '☆ ' + item.title; button.title = item.url; button.onclick = () => navigate(item.url); container.append(button); }
  const label = document.createElement('span'); label.textContent = items.length ? '' : 'Seus favoritos aparecem aqui'; container.append(label);
}
function canonicalUrl(value) { try { return new URL(value).href; } catch { return value; } }
function updateBookmarkButton() {
  const tab = active(), button = $('#bookmark');
  const saved = !!tab?.url && readList('cherry.bookmarks').some(item => canonicalUrl(item.url) === canonicalUrl(tab.url));
  button.textContent = saved ? '★' : '☆';
  button.classList.toggle('is-saved', saved);
  button.setAttribute('aria-pressed', String(saved));
  button.title = saved ? 'Remover dos favoritos' : 'Adicionar favorito';
}
function renderHome() {
  const container = $('#quickLinks'); container.replaceChildren();
  for (const item of readList('cherry.bookmarks').slice(0, 8)) { const button = document.createElement('button'); button.className = 'quick-link'; button.innerHTML = '<span>☆</span><small></small>'; button.querySelector('small').textContent = item.title; button.onclick = () => navigate(item.url); container.append(button); }
}
function applyAppearance() {
  document.documentElement.dataset.theme = localStorage.getItem('cherry.theme') || 'cherry';
  $('#homeView').classList.toggle('no-cover', localStorage.getItem('cherry.cover') === 'false');
  $('#bookmarkBar').classList.toggle('hidden', localStorage.getItem('cherry.showBookmarks') === 'false');
}
function updateAdBlockButton() {
  const enabled = adBlockEnabled(), button = $('#adBlockBtn');
  button.classList.toggle('is-enabled', enabled);
  button.setAttribute('aria-pressed', String(enabled));
  button.title = enabled ? 'Bloqueador de anúncios: ativado' : 'Bloqueador de anúncios: desativado';
}
async function toggleAdBlock() {
  const enabled = !adBlockEnabled();
  localStorage.setItem('cherry.adBlock', String(enabled));
  updateAdBlockButton();
  try {
    await Promise.all(state.tabs.filter(tab => tab.ready).map(tab => invoke('page_action', { label: tab.id, action: 'adblock', url: enabled ? 'on' : 'off' })));
    toast(enabled ? 'Bloqueador ativado' : 'Bloqueador desativado');
  } catch (error) { toast(error?.message || 'Não foi possível atualizar o bloqueador'); }
}
async function setBounds(tab) {
  if (!tab?.ready) return;
  const top = $('#chrome').getBoundingClientRect().height;
  await invoke('set_page_bounds', { label: tab.id, x: 0, y: top, width: window.innerWidth, height: Math.max(100, window.innerHeight - top) });
}
async function updateMenuCutout() {
  const tab = active(), menu = $('#menu');
  if (menu.classList.contains('hidden') || state.panel || !tab?.ready) return;
  const menuRect = menu.getBoundingClientRect();
  const pageTop = $('#chrome').getBoundingClientRect().height;
  const cutoutTop = Math.max(menuRect.top, pageTop);
  await invoke('set_page_cutout', {
    x: menuRect.left, y: cutoutTop - pageTop, width: menuRect.width,
    height: Math.max(0, menuRect.bottom - cutoutTop), enabled: true
  });
  state.menuCutout = true;
}
async function activate(id) {
  const tab = state.tabs.find(item => item.id === id); if (!tab) return;
  await closeMenu(false);
  state.active = id; state.private = tab.private; state.panel = null;
  $('#panelHost').classList.add('hidden'); $('#homeView').classList.toggle('hidden', !!tab.url);
  $('#address').value = tab.url || ''; $('#privacyMark').textContent = tab.private ? '◉' : '◈'; $('#privacyMark').classList.toggle('private', tab.private);
  updateBookmarkButton();
  await Promise.all(state.tabs.filter(other => other.ready && other.id !== id).map(other => invoke('set_page_visible', { label: other.id, visible: false })));
  await setBounds(tab);
  if (tab.ready) {
    await invoke('set_page_visible', { label: tab.id, visible: true });
    await invoke('focus_page', { label: tab.id });
  }
  renderTabs();
}
async function closeMenu(restorePage = true) {
  const menu = $('#menu');
  const wasOpen = !menu.classList.contains('hidden');
  if (state.menuCutout && active()?.ready) {
    await invoke('set_page_cutout', { x: 0, y: 0, width: 0, height: 0, enabled: false });
    state.menuCutout = false;
  }
  if (!wasOpen) return;
  menu.classList.add('hidden'); $('#menuBackdrop').classList.add('hidden'); document.body.classList.remove('menu-open');
  document.documentElement.classList.remove('menu-open', 'menu-overlay');
  if (!restorePage || state.panel) return;
  const tab = active();
  $('#homeView').classList.toggle('hidden', !!tab?.url);
  if (tab?.ready) {
    await invoke('set_page_visible', { label: tab.id, visible: true });
    await setBounds(tab);
    await invoke('focus_page', { label: tab.id });
  }
  else if (tab) await setBounds(tab);
}
async function toggleMenu() {
  if (!$('#menu').classList.contains('hidden')) return closeMenu();
  if (!state.panel) {
    $('#panelHost').classList.add('hidden');
    const tab = active(); $('#homeView').classList.toggle('hidden', !!tab?.url);
  }
  const tab = active(), overlay = !state.panel && !!tab?.ready;
  document.documentElement.classList.add('menu-open');
  $('#menuBackdrop').classList.remove('hidden'); $('#menu').classList.remove('hidden');
  if (overlay) {
    await new Promise(requestAnimationFrame);
    await updateMenuCutout();
  }
}
async function addTab(url = '', privateMode = false) {
  const id = 'tab_' + (++state.seq), tab = { id, title: privateMode ? 'Privada' : 'Nova aba', url: '', private: privateMode, ready: false };
  state.tabs.push(tab); renderTabs(true); await activate(id); if (url) await navigate(url); return tab;
}
async function navigate(input) {
  const tab = active(); if (!tab) return;
  const returningFromPanel = !!state.panel;
  const url = searchUrl(input); tab.url = url; $('#address').value = url; $('#homeView').classList.add('hidden'); $('#panelHost').classList.add('hidden'); state.panel = null;
  updateBookmarkButton();
  if (tab.ready) {
    await invoke('page_action', { label: tab.id, action: 'go', url });
    if (returningFromPanel) {
      await setBounds(tab);
      await invoke('set_page_visible', { label: tab.id, visible: true });
      await invoke('focus_page', { label: tab.id });
    }
  }
  else { await invoke('create_page', { label: tab.id, url, x: 0, y: 0, width: 100, height: 100, incognito: tab.private, adBlock: adBlockEnabled(), visible: true }); tab.ready = true; await setBounds(tab); }
  if (!tab.private) { const history = readList('cherry.history'); history.unshift({ url, at: Date.now() }); save('cherry.history', history.slice(0, 500)); }
}
async function closeTab(id) {
  const index = state.tabs.findIndex(tab => tab.id === id); if (index < 0) return;
  if (id === state.active && state.menuCutout) await closeMenu(false);
  if (state.tabs[index].ready) await invoke('close_page', { label: id }); state.tabs.splice(index, 1);
  if (!state.tabs.length) { state.active = null; await addTab(); } else if (state.active === id) await activate(state.tabs[Math.max(0, index - 1)].id);
  renderTabs();
}
function addListItems(container, items, kind) {
  if (!items.length) { container.textContent = 'Ainda não há itens.'; return; }
  for (const item of items) {
    const row = document.createElement('button'); row.className = 'list-row'; row.innerHTML = '<strong></strong><small></small>';
    row.querySelector('strong').textContent = item.title || item.url || item.name || item.path || 'Download';
    row.querySelector('small').textContent = kind === 'history' && item.at ? new Date(item.at).toLocaleString('pt-BR') : item.url || item.path || item.name || '';
    row.onclick = () => item.url && navigate(item.url); container.append(row);
  }
}
async function showPanel(which) {
  state.panel = which; await closeMenu(false);
  await Promise.all(state.tabs.filter(tab => tab.ready).map(tab => invoke('set_page_visible', { label: tab.id, visible: false })));
  $('#homeView').classList.add('hidden'); const host = $('#panelHost'); host.classList.remove('hidden'); host.replaceChildren();
  const titles = { history: 'Histórico', bookmarks: 'Favoritos', downloads: 'Downloads', settings: 'Configurações', personalize: 'Personalizar', about: 'Sobre o Cherry Browser' };
  const heading = document.createElement('div'); heading.className = 'panel-heading'; heading.innerHTML = '<h2></h2><button class="panel-close">×</button>';
  heading.querySelector('h2').textContent = titles[which]; heading.querySelector('button').onclick = () => activate(state.active); host.append(heading);
  const body = document.createElement('div'); body.className = 'panel-body'; host.append(body);
  if (which === 'history' || which === 'bookmarks' || which === 'downloads') {
    const key = { history: 'cherry.history', bookmarks: 'cherry.bookmarks', downloads: 'cherry.downloads' }[which]; addListItems(body, readList(key), which);
  } else if (which === 'settings') {
    body.innerHTML = `<label>Pesquisa padrão<select id="searchEngine"><option value="google">Google</option><option value="bing">Microsoft Bing (Edge)</option><option value="brave">Brave Search</option><option value="duckduckgo">DuckDuckGo</option></select></label><label class="toggle"><input id="adBlockToggle" type="checkbox"> Bloquear anúncios e rastreadores conhecidos</label><label class="toggle"><input id="showBookmarks" type="checkbox"> Mostrar favoritos abaixo da barra</label><button id="saveSettings" class="primary">Salvar configurações</button><p>O bloqueio é básico e baseado em domínios e elementos comuns. Favoritos e histórico ficam neste dispositivo.</p>`;
    $('#searchEngine').value = localStorage.getItem('cherry.engine') || 'google'; $('#adBlockToggle').checked = adBlockEnabled(); $('#showBookmarks').checked = localStorage.getItem('cherry.showBookmarks') !== 'false';
    $('#saveSettings').onclick = async () => {
      localStorage.setItem('cherry.engine', $('#searchEngine').value); localStorage.setItem('cherry.showBookmarks', $('#showBookmarks').checked); localStorage.setItem('cherry.adBlock', $('#adBlockToggle').checked);
      await Promise.all(state.tabs.filter(tab => tab.ready).map(tab => invoke('page_action', { label: tab.id, action: 'adblock', url: $('#adBlockToggle').checked ? 'on' : 'off' })));
      applyAppearance(); updateAdBlockButton(); toast('Configurações salvas');
    };
  } else if (which === 'personalize') {
    body.innerHTML = `<div class="appearance-preview"><img src="/assets/cherry-cover.svg" alt="Capa de cerejas"><span>Prévia da página inicial</span></div><label>Tema de cores<select id="themeChoice"><option value="cherry">Cereja</option><option value="light">Claro</option><option value="night">Noite</option></select></label><label class="toggle"><input id="coverToggle" type="checkbox"> Mostrar capa de cerejas na página inicial</label><button id="saveAppearance" class="primary">Aplicar personalização</button>`;
    $('#themeChoice').value = localStorage.getItem('cherry.theme') || 'cherry'; $('#coverToggle').checked = localStorage.getItem('cherry.cover') !== 'false';
    $('#saveAppearance').onclick = () => { localStorage.setItem('cherry.theme', $('#themeChoice').value); localStorage.setItem('cherry.cover', $('#coverToggle').checked); applyAppearance(); toast('Personalização aplicada'); };
  } else if (which === 'about') {
    body.innerHTML = `<img class="about-mark" src="/assets/cherry-logo.svg" alt="Logo de cereja"><h3>Cherry Browser</h3><p>by Leo e Vini</p><p><span class="alpha-badge">ALPHA</span> · Versão ${state.version}</p><p>Navegador leve para Windows usando o WebView2 do Microsoft Edge.</p><p>Interface própria com tema de cerejas.</p><button id="checkUpdates" class="primary">↻ Verificar atualizações</button><p id="updateStatus" class="update-status" aria-live="polite">O canal público de atualizações ainda não foi configurado.</p>`;
    $('#checkUpdates').onclick = async event => {
      const button = event.currentTarget, status = $('#updateStatus');
      button.disabled = true; status.textContent = 'Verificando a versão instalada…';
      try {
        state.version = await invoke('app_version');
        status.textContent = `Esta instalação está na versão ${state.version}-alpha. Não há repositório público configurado para comparar versões.`;
      } catch { status.textContent = 'Não foi possível ler a versão instalada.'; }
      finally { button.disabled = false; }
    };
  } else {
    body.innerHTML = `<h3>${titles[which] || 'Cherry Browser'}</h3>`;
  }
}
async function goAddress() { await navigate($('#address').value); }
$('#newTab').onclick = () => addTab(); $('#privateTab').onclick = async () => { await closeMenu(false); await addTab('', true); };
$('#address').addEventListener('keydown', event => { if (event.key === 'Enter') goAddress(); });
$('#homeSearch').addEventListener('keydown', event => { if (event.key === 'Enter') navigate($('#homeSearch').value); }); $('#homeGo').onclick = () => navigate($('#homeSearch').value);
$('#back').onclick = () => active()?.ready && invoke('page_action', { label: state.active, action: 'back' });
$('#forward').onclick = () => active()?.ready && invoke('page_action', { label: state.active, action: 'forward' });
$('#reload').onclick = () => active()?.ready ? invoke('page_action', { label: state.active, action: 'reload' }) : activate(state.active);
$('#home').onclick = async () => { const tab = active(); if (tab) { await closeMenu(false); state.panel = null; $('#panelHost').classList.add('hidden'); if (tab.ready) await invoke('close_page', { label: tab.id }); tab.ready = false; tab.url = ''; tab.title = 'Nova aba'; $('#address').value = ''; $('#homeView').classList.remove('hidden'); renderTabs(); } };
$('#bookmark').onclick = () => { const tab = active(); if (!tab?.url) return toast('Abra uma página primeiro'); const items = readList('cherry.bookmarks'), index = items.findIndex(item => canonicalUrl(item.url) === canonicalUrl(tab.url)); if (index >= 0) { items.splice(index, 1); save('cherry.bookmarks', items); renderBookmarks(); renderHome(); updateBookmarkButton(); toast('Favorito removido'); return; } items.unshift({ url: tab.url, title: tab.title || tab.url }); save('cherry.bookmarks', items); renderBookmarks(); renderHome(); updateBookmarkButton(); toast('Favorito salvo'); };
$('#downloads').onclick = () => showPanel('downloads'); $('#favorites').onclick = () => showPanel('bookmarks'); $('#adBlockBtn').onclick = toggleAdBlock; $('#menuBtn').onclick = () => toggleMenu(); $('#menuBackdrop').onclick = () => closeMenu();
$('#menu').querySelectorAll('[data-panel]').forEach(button => button.onclick = () => showPanel(button.dataset.panel));
$('#clearData').onclick = async () => { try { await invoke('clear_page_data', { labels: state.tabs.filter(tab => tab.ready).map(tab => tab.id) }); localStorage.removeItem('cherry.history'); await closeMenu(); toast('Histórico, cookies e cache foram limpos'); } catch (error) { toast(error?.message || 'Não foi possível limpar os dados'); } };
let resizeTimer;
addEventListener('resize', () => {
  clearTimeout(resizeTimer);
  resizeTimer = setTimeout(() => {
    setBounds(active()).then(updateMenuCutout).catch(error => toast(error?.message || 'Não foi possível ajustar a página'));
  }, 70);
});
document.addEventListener('keydown', event => { if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 't') { event.preventDefault(); addTab(); } if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'l') { event.preventDefault(); $('#address').focus(); $('#address').select(); } if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'w' && active()) { event.preventDefault(); closeTab(state.active); } if (event.key === 'Escape') closeMenu(); });
listen('cherry:title', event => { const tab = state.tabs.find(item => item.id === event.payload.label); if (tab) { tab.title = event.payload.title || tab.url; renderTabs(); renderBookmarks(); } });
listen('cherry:navigation', event => { const tab = state.tabs.find(item => item.id === event.payload.label); if (tab) { tab.url = event.payload.url; if (tab.id === state.active) { $('#address').value = tab.url; updateBookmarkButton(); } } });
listen('cherry:shortcut', event => { if (event.payload.action === 'new-tab') addTab(); else if (event.payload.action === 'focus-address') { $('#address').focus(); $('#address').select(); } else if (event.payload.action === 'close-tab' && active()) closeTab(state.active); });
listen('cherry:new-window', event => { const parent = state.tabs.find(tab => tab.id === event.payload.label); addTab(event.payload.url, !!parent?.private); });
listen('cherry:download', event => { const download = event.payload, items = readList('cherry.downloads'), existing = items.find(item => item.url === download.url && item.state === 'started'); if (existing) { existing.state = download.state; existing.path = download.path || existing.path; } else items.unshift({ url: download.url, path: download.path, state: download.state, at: Date.now() }); if (!download.incognito) save('cherry.downloads', items.slice(0, 500)); if (download.state === 'finished') toast('Download concluído'); });
listen('cherry:page-load', event => { if (event.payload.state === 'finished' && event.payload.label === state.active) $('#homeView').classList.add('hidden'); });
applyAppearance(); updateAdBlockButton(); renderTabs(); renderBookmarks(); renderHome(); invoke('app_version').then(version => state.version = version).catch(() => {}); addTab();


