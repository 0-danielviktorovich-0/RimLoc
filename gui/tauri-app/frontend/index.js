const { invoke } = window.__TAURI__.invoke;

function $(id) { return document.getElementById(id); }

function setTab(active) {
  document.querySelectorAll('section.tab').forEach(s => s.classList.remove('active'));
  document.getElementById(active).classList.add('active');
}

document.querySelectorAll('nav button[data-tab]').forEach(btn => {
  btn.addEventListener('click', () => setTab(btn.dataset.tab));
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

