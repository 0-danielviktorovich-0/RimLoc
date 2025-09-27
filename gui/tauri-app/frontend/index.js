const { invoke } = window.__TAURI__.invoke;

// --- i18n ---
const I18N = {
  en: {
    tab_start: 'Start', tab_validate: 'Validate', tab_diff: 'Diff', tab_import: 'Import/Build', tab_annotate: 'Annotate', tab_morph: 'Morph', tab_lang: 'Lang Update', tab_tools: 'Tools', tab_settings: 'Settings',
    hdr_start: 'Start', hdr_validate: 'Validate', hdr_diff: 'Diff', hdr_import: 'Import/Build', hdr_annotate: 'Annotate', hdr_morph: 'Morph', hdr_lang: 'Lang Update', hdr_tools: 'Tools',
    lbl_mod_root: 'Mod root', lbl_src_code: 'Source lang code', lbl_trg_code: 'Target lang code', lbl_po_out: 'PO output', lbl_tm_roots: 'TM roots (comma)', lbl_src_dir: 'Source lang dir', lbl_trg_dir: 'Target lang dir', lbl_po_file: 'PO file', lbl_only_diff: 'Only diff', lbl_game_root: 'Game root', lbl_repo: 'Repo', lbl_branch: 'Branch', lbl_zip: 'Zip', lbl_ui_lang: 'UI:', lbl_out_dir: 'Output dir', lbl_baseline_po: 'Baseline PO', lbl_reports_dir: 'Reports out dir',
    btn_scan: 'Scan', btn_export: 'Export PO', btn_validate: 'Validate', btn_xml_health: 'XML Health', btn_run_diff: 'Run Diff', btn_import_dry: 'Import DRY-RUN', btn_import_apply: 'Apply Import', btn_build_dry: 'Build DRY-RUN', btn_build_apply: 'Apply Build', btn_dry_run: 'Dry-run', btn_apply: 'Apply', btn_run: 'Run', btn_plan_dry: 'Plan Update (DRY)', btn_apply_update: 'Apply Update', btn_schema_dump: 'Dump JSON Schemas', btn_save_json: 'Save JSON', btn_save_reports: 'Save Reports', btn_open_last: 'Open Last Path', btn_read: 'Read', btn_auto: 'Auto', btn_copy_out: 'Copy', btn_clear_out: 'Clear',
    tab_logs: 'Logs', hdr_logs: 'Logs', lbl_log_path: 'Log file path', btn_read: 'Read', btn_auto: 'Auto',
    confirm_apply_import: 'Apply import to mod files? Backups recommended.',
    confirm_apply_build: 'Create/overwrite translation mod at output path?',
    confirm_apply_lang: 'Apply official localization update to game? Backup existing folder?',
    confirm_apply_annotate: 'Apply annotate changes to XML files?',
    ok_saved: 'Done.',
  },
  ru: {
    tab_start: 'Старт', tab_validate: 'Проверка', tab_diff: 'Дифф', tab_import: 'Импорт/Сборка', tab_annotate: 'Аннотация', tab_morph: 'Морфология', tab_lang: 'Обновление', tab_tools: 'Инструменты', tab_settings: 'Настройки',
    hdr_start: 'Старт', hdr_validate: 'Проверка', hdr_diff: 'Дифф', hdr_import: 'Импорт/Сборка', hdr_annotate: 'Аннотация', hdr_morph: 'Морфология', hdr_lang: 'Обновление локализации', hdr_tools: 'Инструменты',
    lbl_mod_root: 'Корень мода', lbl_src_code: 'Код исходного языка', lbl_trг_code: 'Код целевого языка', lbl_po_out: 'Выходной PO', lbl_tm_roots: 'TM базы (через запятую)', lbl_src_dir: 'Папка исходного языка', lbl_trg_dir: 'Папка целевого языка', lbl_po_file: 'Файл PO', lbl_only_diff: 'Только отличия', lbl_game_root: 'Корень игры', lbl_repo: 'Репозиторий', lbl_branch: 'Ветка', lbl_zip: 'ZIP', lbl_ui_lang: 'Интерфейс:', lbl_out_dir: 'Папка вывода', lbl_baseline_po: 'Базовый PO', lbl_reports_dir: 'Папка отчётов',
    btn_scan: 'Сканировать', btn_export: 'Экспорт PO', btn_validate: 'Проверить', btn_xml_health: 'XML здоровье', btn_run_diff: 'Запустить Diff', btn_import_dry: 'Импорт DRY-RUN', btn_import_apply: 'Применить импорт', btn_build_dry: 'Сборка DRY-RUN', btn_build_apply: 'Применить сборку', btn_dry_run: 'DRY-RUN', btn_apply: 'Применить', btn_run: 'Запустить', btn_plan_dry: 'План (DRY)', btn_apply_update: 'Применить обновление', btn_schema_dump: 'Выгрузить JSON схемы', btn_save_json: 'Сохранить JSON', btn_save_reports: 'Сохранить отчёты', btn_open_last: 'Открыть путь', btn_read: 'Прочитать', btn_auto: 'Авто', btn_copy_out: 'Копировать', btn_clear_out: 'Очистить',
    tab_logs: 'Логи', hdr_logs: 'Логи', lbl_log_path: 'Путь к лог‑файлу',
    confirm_apply_import: 'Применить импорт к файлам мода? Рекомендуется бэкап.',
    confirm_apply_build: 'Создать/перезаписать мод-перевод по указанному пути?',
    confirm_apply_lang: 'Обновить локализацию в игре? Сделать резервную копию существующей папки?',
    confirm_apply_annotate: 'Применить аннотирование к XML файлам?',
    ok_saved: 'Готово.',
  }
};
let UI_LANG = localStorage.getItem('ui-lang') || 'en';
document.addEventListener('DOMContentLoaded', () => {
  $('ui-lang').value = UI_LANG;
});
$('ui-lang').addEventListener('change', () => {
  UI_LANG = $('ui-lang').value;
  localStorage.setItem('ui-lang', UI_LANG);
  applyI18n();
});
function t(k) { return (I18N[UI_LANG] && I18N[UI_LANG][k]) || I18N.en[k] || k; }

function applyI18n() {
  document.querySelectorAll('[data-i18n]').forEach(el => {
    const k = el.getAttribute('data-i18n');
    if (!k) return;
    const isInput = el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT';
    if (!isInput) {
      // replace only text nodes (keep children like inputs/buttons)
      el.childNodes.forEach(n => { if (n.nodeType === Node.TEXT_NODE) n.textContent = t(k); });
    }
  });
  // placeholders
  document.querySelectorAll('[data-i18n-placeholder]').forEach(el => {
    const k = el.getAttribute('data-i18n-placeholder');
    if (k) el.setAttribute('placeholder', t(k));
  });
  // titles
  document.querySelectorAll('[data-i18n-title]').forEach(el => {
    const k = el.getAttribute('data-i18n-title');
    if (k) el.setAttribute('title', t(k));
  });
}
applyI18n();

function $(id) { return document.getElementById(id); }

function setTab(active) {
  document.querySelectorAll('section.tab').forEach(s => s.classList.remove('active'));
  document.getElementById(active).classList.add('active');
}

// overlay & toasts helpers
function showOverlay(text) {
  const o = $('overlay');
  const t = $('overlay-text');
  if (t) t.textContent = text || 'Working…';
  if (o) o.classList.add('show');
}
function hideOverlay() { const o = $('overlay'); if (o) o.classList.remove('show'); setOverlayProgress(0); }
let toastTimer;
function showToast(text, timeout=2000) {
  const el = $('toast');
  if (!el) return;
  el.textContent = text;
  el.classList.add('show');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(()=> el.classList.remove('show'), timeout);
}
function setOverlayProgress(pct) {
  const bar = $('overlay-bar');
  if (bar) bar.style.width = Math.max(0, Math.min(100, pct)) + '%';
}

document.querySelectorAll('nav button[data-tab]').forEach(btn => {
  btn.addEventListener('click', () => {
    document.querySelectorAll('nav button[data-tab]').forEach(b => b.classList.remove('active'));
    btn.classList.add('active');
    setTab(btn.dataset.tab);
  });
});

$('btn-scan').addEventListener('click', async () => {
  const root = $('start-mod-root').value.trim();
  $('start-output').textContent = 'Scanning...';
  try {
    const units = await invoke('api_scan', { root });
    $('start-output').textContent = JSON.stringify({ count: units.length, sample: units.slice(0, 10) }, null, 2);
  } catch (e) { $('start-output').textContent = String(e); }
});

$('btn-export').addEventListener('click', async () => {
  const root = $('start-mod-root').value.trim();
  const outPo = $('start-po-out').value.trim();
  const src = $('start-src').value.trim() || null;
  const trg = $('start-trg').value.trim() || null;
  const tm = $('start-tm-roots').value.trim();
  const tmv = tm ? tm.split(',').map(s => s.trim()).filter(Boolean) : null;
  $('start-output').textContent = 'Exporting PO...';
  try {
    await invoke('api_export_po', { root, outPo, lang: trg, sourceLang: src, sourceLangDir: null, tmRoots: tmv });
    $('start-output').textContent = `PO saved to ${outPo}`;
  } catch (e) { $('start-output').textContent = String(e); }
});

$('btn-validate').addEventListener('click', async () => {
  const root = $('val-mod-root').value.trim();
  const src = $('val-src').value.trim() || null;
  const srcDir = $('val-src-dir').value.trim() || null;
  $('validate-output').textContent = 'Validating...';
  try {
    const msgs = await invoke('api_validate', { root, sourceLang: src, sourceLangDir: srcDir });
    $('validate-output').textContent = JSON.stringify({ issues: msgs.length, messages: msgs.slice(0, 100) }, null, 2);
  } catch (e) { $('validate-output').textContent = String(e); }
});

$('btn-xml-health').addEventListener('click', async () => {
  const root = $('val-mod-root').value.trim();
  $('validate-output').textContent = 'Scanning XML health...';
  try {
    const rep = await invoke('api_xml_health', { root, langDir: null });
    $('validate-output').textContent = JSON.stringify(rep, null, 2);
  } catch (e) { $('validate-output').textContent = String(e); }
});

// Save Validate/Health JSON to file
document.getElementById('btn-validate-save')?.addEventListener('click', async () => {
  const path = await window.__TAURI__.dialog.save({ defaultPath: './logs/validate.json' });
  if (!path) return;
  try {
    await invoke('api_save_text', { path, contents: $('validate-output').textContent });
    alert('Saved: ' + path);
  } catch (e) { alert(String(e)); }
});

$('btn-diff').addEventListener('click', async () => {
  const root = $('diff-mod-root').value.trim();
  const src = $('diff-src').value.trim() || null;
  const srcDir = $('diff-src-dir').value.trim() || null;
  const trg = $('diff-trg').value.trim() || null;
  const trgDir = $('diff-trg-dir').value.trim() || null;
  const baselinePo = document.getElementById('diff-baseline') ? ($('diff-baseline').value.trim() || null) : null;
  $('diff-output').textContent = 'Diffing...';
  try {
    const out = await invoke('api_diff_xml', { root, sourceLang: src, sourceLangDir: srcDir, lang: trg, langDir: trgDir, baselinePo });
    const summary = { changed: out.changed.length, only_in_translation: out.only_in_translation.length, only_in_mod: out.only_in_mod.length };
    $('diff-output').textContent = JSON.stringify({ summary, ...out }, null, 2);
  } catch (e) { $('diff-output').textContent = String(e); }
});

// Save Diff reports
document.getElementById('btn-diff-save')?.addEventListener('click', async () => {
  const root = $('diff-mod-root').value.trim();
  const src = $('diff-src').value.trim() || null;
  const srcDir = $('diff-src-dir').value.trim() || null;
  const trg = $('diff-trg').value.trim() || null;
  const trgDir = $('diff-trg-dir').value.trim() || null;
  const baselinePo = document.getElementById('diff-baseline') ? ($('diff-baseline').value.trim() || null) : null;
  const outDir = document.getElementById('diff-out-dir') ? ($('diff-out-dir').value.trim() || './logs/diff') : './logs/diff';
  $('diff-output').textContent = 'Saving reports...';
  try {
    const p = await invoke('api_diff_save_reports', { root, sourceLang: src, sourceLangDir: srcDir, lang: trg, langDir: trgDir, baselinePo, outDir });
    $('diff-output').textContent = `Reports saved to ${p}`;
    showToast('Reports saved');
  } catch (e) { $('diff-output').textContent = String(e); }
});

$('btn-import-dry').addEventListener('click', async () => {
  const po = $('imp-po').value.trim();
  const root = $('imp-mod-root').value.trim();
  const trg = $('imp-trg').value.trim() || null;
  const trgDir = $('imp-trg-dir').value.trim() || null;
  const onlyDiff = $('imp-only-diff').checked;
  $('import-output').textContent = 'Import dry-run...';
  try {
    const plan = await invoke('api_import_po_dry', { po, modRoot: root, lang: trg, langDir: trgDir, keepEmpty: false, singleFile: false, gameVersion: null, onlyDiff });
    $('import-output').textContent = JSON.stringify(plan, null, 2);
    showToast('Import plan ready');
  } catch (e) { $('import-output').textContent = String(e); }
});

$('btn-build-dry').addEventListener('click', async () => {
  const po = $('imp-po').value.trim() || null;
  const outMod = './logs/RimLoc-Translation';
  const lang = $('imp-trg').value.trim();
  $('import-output').textContent = 'Build dry-run...';
  try {
    const plan = await invoke('api_build_mod_dry', { po, outMod, lang, fromRoot: null, fromGameVersion: null, name: null, packageId: null, rwVersion: null, langDir: null, dedupe: true });
    $('import-output').textContent = JSON.stringify(plan, null, 2);
    showToast('Build plan ready');
  } catch (e) { $('import-output').textContent = String(e); }
});

// Save Import/Build JSON
document.getElementById('btn-import-save')?.addEventListener('click', async () => {
  const path = await window.__TAURI__.dialog.save({ defaultPath: './logs/import.json' });
  if (!path) return;
  try { await invoke('api_save_text', { path, contents: $('import-output').textContent }); showToast('Saved'); } catch (e) { alert(String(e)); }
});

// Archive Build output
document.getElementById('btn-build-archive')?.addEventListener('click', async () => {
  const dir = window.LAST_PATH || './logs/RimLoc-Translation';
  const zip = await window.__TAURI__.dialog.save({ defaultPath: './logs/RimLoc-Translation.zip' });
  if (!zip) return;
  try { const out = await invoke('api_zip_dir', { dir, outZip: zip }); showToast('Archived: ' + out); } catch (e) { alert(String(e)); }
});

// Archive Language dir after import
document.getElementById('btn-import-archive')?.addEventListener('click', async () => {
  const root = $('imp-mod-root').value.trim();
  const trg = $('imp-trg').value.trim() || null;
  const trgDir = $('imp-trg-dir').value.trim() || (trg ? (trg.toLowerCase()==='ru' ? 'Russian' : '') : '');
  if (!root || !trgDir) { alert('Set mod root and target lang folder'); return; }
  const dir = `${root.replace(/\\$/,'')}/Languages/${trgDir}`;
  const zip = await window.__TAURI__.dialog.save({ defaultPath: `./logs/${trgDir}.zip` });
  if (!zip) return;
  try { const out = await invoke('api_zip_dir', { dir, outZip: zip }); showToast('Archived: ' + out); } catch (e) { alert(String(e)); }
});

// Apply Import
$('btn-import-apply').addEventListener('click', async () => {
  if (!confirm(t('confirm_apply_import'))) return;
  const po = $('imp-po').value.trim();
  const root = $('imp-mod-root').value.trim();
  const trg = $('imp-trg').value.trim() || null;
  const trgDir = $('imp-trg-dir').value.trim() || null;
  const onlyDiff = $('imp-only-diff').checked;
  $('import-output').textContent = 'Applying import...'; showOverlay('Importing…'); setOverlayProgress(0);
  try {
    const unlisten = await window.__TAURI__.event.listen('import_progress', (ev) => {
      const d = ev.payload; const pct = d.total>0? Math.round((d.current/d.total)*100):0; setOverlayProgress(pct);
    });
    const sum = await invoke('api_import_po_apply', { po, modRoot: root, lang: trg, langDir: trgDir, keepEmpty: false, singleFile: false, incremental: true, onlyDiff: onlyDiff, report: true, backup: true });
    $('import-output').textContent = JSON.stringify(sum, null, 2);
    unlisten(); showToast('Import done');
  } catch (e) { $('import-output').textContent = String(e); }
  finally { hideOverlay(); }
});

// Apply Build
$('btn-build-apply').addEventListener('click', async () => {
  if (!confirm(t('confirm_apply_build'))) return;
  const po = $('imp-po').value.trim() || null;
  const outMod = './logs/RimLoc-Translation';
  const lang = $('imp-trg').value.trim();
  $('import-output').textContent = 'Building mod...'; showOverlay('Building…'); setOverlayProgress(0);
  try {
    const unlisten = await window.__TAURI__.event.listen('build_progress', (ev) => { const d = ev.payload; const pct = d.total>0? Math.round((d.current/d.total)*100):0; setOverlayProgress(pct); });
    const out = await invoke('api_build_mod_apply', { po, outMod, lang, fromRoot: null, fromGameVersion: null, name: null, packageId: null, rwVersion: null, langDir: null, dedupe: true });
    $('import-output').textContent = `Built at ${out}`;
    LAST_PATH = out;
    unlisten(); showToast('Build done');
  } catch (e) { $('import-output').textContent = String(e); }
  finally { hideOverlay(); }
});

$('btn-lang-dry').addEventListener('click', async () => {
  const gameRoot = $('lang-game-root').value.trim();
  const repo = $('lang-repo').value.trim() || null;
  const branch = $('lang-branch').value.trim() || null;
  const zip = $('lang-zip').value.trim() || null;
  const srcDir = $('lang-src-dir').value.trim() || null;
  const trgDir = $('lang-trg-dir').value.trim() || null;
  $('lang-output').textContent = 'Planning update...';
  try {
    const res = await invoke('api_lang_update_dry', { gameRoot, repo, branch, zip, sourceLangDir: srcDir, targetLangDir: trgDir });
    $('lang-output').textContent = JSON.stringify(res, null, 2);
  } catch (e) { $('lang-output').textContent = String(e); }
});

// Apply Lang Update
$('btn-lang-apply').addEventListener('click', async () => {
  if (!confirm(t('confirm_apply_lang'))) return;
  const gameRoot = $('lang-game-root').value.trim();
  const repo = $('lang-repo').value.trim() || null;
  const branch = $('lang-branch').value.trim() || null;
  const zip = $('lang-zip').value.trim() || null;
  const srcDir = $('lang-src-dir').value.trim() || null;
  const trgDir = $('lang-trg-dir').value.trim() || null;
  $('lang-output').textContent = 'Applying update...';
  try {
    const out = await invoke('api_lang_update_apply', { gameRoot, repo, branch, zip, sourceLangDir: srcDir, targetLangDir: trgDir, backup: true });
    $('lang-output').textContent = `${t('ok_saved')} -> ${out}`;
    LAST_PATH = out;
  } catch (e) { $('lang-output').textContent = String(e); }
});

// Annotate
$('btn-ann-dry').addEventListener('click', async () => {
  const root = $('ann-root').value.trim();
  const src = $('ann-src').value.trim() || null;
  const srcDir = $('ann-src-dir').value.trim() || null;
  const trg = $('ann-trg').value.trim() || null;
  const trgDir = $('ann-trg-dir').value.trim() || null;
  const prefix = $('ann-prefix').value.trim() || null;
  const strip = $('ann-strip').checked;
  $('annotate-output').textContent = 'Planning annotate...';
  try {
    const plan = await invoke('api_annotate_dry', { root, sourceLang: src, sourceLangDir: srcDir, lang: trg, langDir: trgDir, commentPrefix: prefix, strip });
    $('annotate-output').textContent = JSON.stringify(plan, null, 2);
  } catch (e) { $('annotate-output').textContent = String(e); }
});

$('btn-ann-apply').addEventListener('click', async () => {
  if (!confirm(t('confirm_apply_annotate'))) return;
  const root = $('ann-root').value.trim();
  const src = $('ann-src').value.trim() || null;
  const srcDir = $('ann-src-dir').value.trim() || null;
  const trg = $('ann-trg').value.trim() || null;
  const trgDir = $('ann-trg-dir').value.trim() || null;
  const prefix = $('ann-prefix').value.trim() || null;
  const strip = $('ann-strip').checked;
  const backup = $('ann-backup').checked;
  $('annotate-output').textContent = 'Annotating...';
  try {
    const sum = await invoke('api_annotate_apply', { root, sourceLang: src, sourceLangDir: srcDir, lang: trg, langDir: trgDir, commentPrefix: prefix, strip, backup });
    $('annotate-output').textContent = JSON.stringify(sum, null, 2);
  } catch (e) { $('annotate-output').textContent = String(e); }
});

// Save annotate JSON (plan or summary)
document.getElementById('btn-ann-save')?.addEventListener('click', async () => {
  const path = await window.__TAURI__.dialog.save({ defaultPath: './logs/annotate.json' });
  if (!path) return;
  try {
    await invoke('api_save_text', { path, contents: $('annotate-output').textContent });
    alert('Saved: ' + path);
  } catch (e) { alert(String(e)); }
});

// Show app version in footer
document.addEventListener('DOMContentLoaded', async () => {
  try {
    const v = await invoke('api_app_version');
    const el = document.querySelector('#app-version small');
    if (el) el.textContent = `RimLoc GUI v${v} • Tauri shell • Made with ❤️`;
  } catch (e) {}
  // listen update checks
  await window.__TAURI__.event.listen('gui_update_check', (ev) => {
    const d = ev.payload;
    if (d.error) { showToast('Update check error'); return; }
    if (d.current && d.latest && d.latest !== '' && d.latest !== d.current) {
      showToast(`New version available: ${d.latest}`);
    } else {
      showToast('You are up to date');
    }
  });
});

// --- persistence of inputs ---
function bindPersistInput(id) {
  const el = $(id);
  const key = `rimloc-ui:${id}`;
  const saved = localStorage.getItem(key);
  if (saved !== null) el.value = saved;
  el.addEventListener('input', () => localStorage.setItem(key, el.value));
}

['start-mod-root','start-src','start-trg','start-po-out','start-tm-roots',
 'val-mod-root','val-src','val-src-dir',
 'diff-mod-root','diff-src','diff-src-dir','diff-trg','diff-trg-dir',
 'imp-po','imp-mod-root','imp-trg','imp-trg-dir',
 'ann-root','ann-src','ann-src-dir','ann-trg','ann-trg-dir','ann-prefix',
 'lang-game-root','lang-repo','lang-branch','lang-zip','lang-src-dir','lang-trg-dir']
 .forEach(bindPersistInput);

// pickers
document.querySelectorAll('.pick-btn').forEach(btn => {
  btn.addEventListener('click', async () => {
    const field = btn.getAttribute('data-for');
    const mode = btn.getAttribute('data-pick');
    try {
      let selected;
      if (mode === 'dir') {
        selected = await window.__TAURI__.dialog.open({ directory: true, multiple: false });
      } else if (mode === 'file') {
        selected = await window.__TAURI__.dialog.open({ multiple: false });
      } else if (mode === 'savefile') {
        selected = await window.__TAURI__.dialog.save({ defaultPath: $(field).value || undefined });
      }
      if (selected) { $(field).value = selected; localStorage.setItem(`rimloc-ui:${field}`, selected); }
    } catch (e) { /* ignore */ }
  });
});

// Open last path
let LAST_PATH = null;
$('btn-open-path').addEventListener('click', async () => {
  if (!LAST_PATH) return;
  try { await window.__TAURI__.shell.open(LAST_PATH); } catch (e) { /* ignore */ }
});

// Morph run
$('btn-morph-run').addEventListener('click', async () => {
  const root = $('morph-root').value.trim();
  const provider = $('morph-provider').value;
  const lang = $('morph-trg').value.trim() || null;
  const langDir = $('morph-trg-dir').value.trim() || null;
  const filterKeyRegex = $('morph-filter').value.trim() || null;
  const limit = parseInt($('morph-limit').value.trim() || '0', 10) || null;
  const timeoutMs = parseInt($('morph-timeout').value.trim() || '0', 10) || null;
  const cacheSize = parseInt($('morph-cache').value.trim() || '0', 10) || null;
  const pymorphyUrl = $('morph-pym-url').value.trim() || null;
  $('morph-output').textContent = 'Running morph...';
  try {
    const res = await invoke('api_morph', { root, provider, lang, langDir, filterKeyRegex, limit, gameVersion: null, timeoutMs, cacheSize, pymorphyUrl });
    $('morph-output').textContent = JSON.stringify(res, null, 2);
  } catch (e) { $('morph-output').textContent = String(e); }
});

// Schema dump
$('btn-schema-dump').addEventListener('click', async () => {
  const outDir = $('tools-out-dir').value.trim();
  $('tools-output').textContent = 'Dumping schemas...';
  try {
    const p = await invoke('api_schema_dump', { outDir });
    $('tools-output').textContent = `Saved to ${p}`;
    LAST_PATH = p;
  } catch (e) { $('tools-output').textContent = String(e); }
});

// Logs tab
$('btn-logs-read').addEventListener('click', async () => {
  const path = $('logs-path').value.trim() || './logs/rimloc.log';
  const lines = parseInt($('logs-lines').value.trim() || '200', 10) || 200;
  try {
    const text = await invoke('api_read_log_tail', { path, lines });
    $('logs-output').textContent = text;
  } catch (e) { $('logs-output').textContent = String(e); }
});

let LOGS_TIMER;
$('btn-logs-auto').addEventListener('click', async () => {
  if (LOGS_TIMER) { clearInterval(LOGS_TIMER); LOGS_TIMER = null; showToast('Auto off'); return; }
  showToast('Auto on');
  LOGS_TIMER = setInterval(() => { document.getElementById('btn-logs-read').click(); }, 2000);
});

// Lang update with progress (download+plan)
document.getElementById('btn-lang-dry').addEventListener('contextmenu', async (e) => {
  e.preventDefault();
  const gameRoot = $('lang-game-root').value.trim();
  const repo = $('lang-repo').value.trim() || 'Ludeon/RimWorld-ru';
  const branch = $('lang-branch').value.trim() || null;
  const srcDir = $('lang-src-dir').value.trim() || 'Russian';
  const trgDir = $('lang-trg-dir').value.trim() || 'Russian (GitHub)';
  $('lang-output').textContent = 'Downloading & planning...';
  showOverlay('Downloading…');
  try {
    const unlisten = await window.__TAURI__.event.listen('lang_update_progress', (ev) => {
      const d = ev.payload;
      const pct = d.total > 0 ? Math.round((d.downloaded / d.total) * 100) : 0;
      $('overlay-text').textContent = `Downloading… ${pct}%`;
      setOverlayProgress(pct);
    });
    const res = await invoke('api_lang_update_download_and_plan', { gameRoot, repo, branch, sourceLangDir: srcDir, targetLangDir: trgDir });
    $('lang-output').textContent = JSON.stringify(res, null, 2);
    unlisten();
    showToast('Plan ready');
  } catch (e) { $('lang-output').textContent = String(e); }
  finally { hideOverlay(); }
});
    // placeholders
    ph_mod_root: '/path/to/Mods/MyMod', ph_po_out: './logs/MyMod.po', ph_tm_roots: '/path/Languages/Russian,/other/root', ph_src_dir: 'English (optional)', ph_trg_dir: 'Russian (optional)', ph_po_file: './logs/MyMod.po', ph_baseline: './prev.po (optional)', ph_out_dir: './logs/diff', ph_pym_url: 'http://localhost:5000', ph_game_root: '/games/RimWorld', ph_branch: '(default)', ph_zip: './RimWorld-ru.zip (optional)', ph_log_path: './logs/rimloc.log',
    // tooltips
    tt_pick_dir: 'Choose directory', tt_pick_file: 'Choose file', tt_pick_save: 'Choose save file', tt_mod_root: 'Path to mod root', tt_src_code: 'e.g., en', tt_trg_code: 'e.g., ru', tt_po_out: 'Where to save PO', tt_tm_roots: 'Comma-separated TM roots', tt_src_dir: 'Folder name of source language', tt_trg_dir: 'Folder name of target language', tt_game_root: 'RimWorld installation root', tt_repo: 'owner/name', tt_branch: 'Branch name', tt_out_dir: 'Output directory', tt_log_path: 'Path to rimloc log file',
    // placeholders
    ph_mod_root: '/path/to/Mods/MyMod', ph_po_out: './logs/MyMod.po', ph_tm_roots: '/path/Languages/Russian,/other/root', ph_src_dir: 'English (опционально)', ph_trg_dir: 'Russian (опционально)', ph_po_file: './logs/MyMod.po', ph_baseline: './prev.po (опционально)', ph_out_dir: './logs/diff', ph_pym_url: 'http://localhost:5000', ph_game_root: '/games/RimWorld', ph_branch: '(по умолчанию)', ph_zip: './RimWorld-ru.zip (опционально)', ph_log_path: './logs/rimloc.log',
    // tooltips
    tt_pick_dir: 'Выбрать папку', tt_pick_file: 'Выбрать файл', tt_pick_save: 'Выбрать файл сохранения', tt_mod_root: 'Путь к корню мода', tt_src_code: 'например, en', tt_trg_code: 'например, ru', tt_po_out: 'Куда сохранить PO', tt_tm_roots: 'TM базы через запятую', tt_src_dir: 'Имя папки исходного языка', tt_trg_dir: 'Имя папки целевого языка', tt_game_root: 'Папка установленной игры', tt_repo: 'owner/name', tt_branch: 'Имя ветки', tt_out_dir: 'Папка вывода', tt_log_path: 'Путь к лог‑файлу rimloc',
