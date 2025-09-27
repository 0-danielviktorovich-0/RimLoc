const { invoke } = window.__TAURI__.invoke;

// --- i18n ---
const I18N = {
  en: {
    confirm_apply_import: 'Apply import to mod files? Backups recommended.',
    confirm_apply_build: 'Create/overwrite translation mod at output path?',
    confirm_apply_lang: 'Apply official localization update to game? Backup existing folder?',
    confirm_apply_annotate: 'Apply annotate changes to XML files?',
    ok_saved: 'Done.',
  },
  ru: {
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
}
applyI18n();

function $(id) { return document.getElementById(id); }

function setTab(active) {
  document.querySelectorAll('section.tab').forEach(s => s.classList.remove('active'));
  document.getElementById(active).classList.add('active');
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

$('btn-diff').addEventListener('click', async () => {
  const root = $('diff-mod-root').value.trim();
  const src = $('diff-src').value.trim() || null;
  const srcDir = $('diff-src-dir').value.trim() || null;
  const trg = $('diff-trg').value.trim() || null;
  const trgDir = $('diff-trg-dir').value.trim() || null;
  $('diff-output').textContent = 'Diffing...';
  try {
    const out = await invoke('api_diff_xml', { root, sourceLang: src, sourceLangDir: srcDir, lang: trg, langDir: trgDir });
    $('diff-output').textContent = JSON.stringify(out, null, 2);
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
  } catch (e) { $('import-output').textContent = String(e); }
});

// Apply Import
$('btn-import-apply').addEventListener('click', async () => {
  if (!confirm(t('confirm_apply_import'))) return;
  const po = $('imp-po').value.trim();
  const root = $('imp-mod-root').value.trim();
  const trg = $('imp-trg').value.trim() || null;
  const trgDir = $('imp-trg-dir').value.trim() || null;
  const onlyDiff = $('imp-only-diff').checked;
  $('import-output').textContent = 'Applying import...';
  try {
    const sum = await invoke('api_import_po_apply', { po, modRoot: root, lang: trg, langDir: trgDir, keepEmpty: false, singleFile: false, incremental: true, onlyDiff: onlyDiff, report: true, backup: true });
    $('import-output').textContent = JSON.stringify(sum, null, 2);
  } catch (e) { $('import-output').textContent = String(e); }
});

// Apply Build
$('btn-build-apply').addEventListener('click', async () => {
  if (!confirm(t('confirm_apply_build'))) return;
  const po = $('imp-po').value.trim() || null;
  const outMod = './logs/RimLoc-Translation';
  const lang = $('imp-trg').value.trim();
  $('import-output').textContent = 'Building mod...';
  try {
    const out = await invoke('api_build_mod_apply', { po, outMod, lang, fromRoot: null, fromGameVersion: null, name: null, packageId: null, rwVersion: null, langDir: null, dedupe: true });
    $('import-output').textContent = `Built at ${out}`;
    LAST_PATH = out;
  } catch (e) { $('import-output').textContent = String(e); }
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
