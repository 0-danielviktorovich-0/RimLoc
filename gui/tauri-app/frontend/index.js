function getTauri() {
  const t = (typeof window !== 'undefined' && window.__TAURI__) ? window.__TAURI__ : undefined;
  if (!getTauri._loggedOnce) {
    getTauri._loggedOnce = true;
    try { console.log('TAURI global:', t); } catch {}
  }
  return t || {};
}

function tauriInvoke(cmd, args) {
  const tauri = getTauri();
  const fn = tauri.invoke || tauri.tauri?.invoke || tauri.core?.invoke;
  if (!fn) { try { console.error('invoke not available', { cmd, args }); } catch {}; throw new Error("Tauri API not available: invoke"); }
  try { console.log('invoke', cmd, args); } catch {}
  try { debugLog("trace", `invoke ${cmd} → ${sanitizeArgs(args)}`, true); } catch {}
  const start = Date.now();
  return fn(cmd, args)
    .then((res) => { try { console.log('invoke ✓', cmd, Date.now()-start, 'ms', res); } catch {}; try { debugLog("trace", `invoke ${cmd} ✓ ${Date.now()-start}ms`, true); } catch {} return res; })
    .catch((e) => { try { console.error('invoke ✗', cmd, Date.now()-start, 'ms', e); } catch {}; try { debugLog("error", `invoke ${cmd} ✗ ${Date.now()-start}ms: ${formatError(e)}`, true); } catch {} throw e; });
}

function sanitizeArgs(args) {
  try {
    const home = (typeof process !== 'undefined' && process.env && process.env.HOME) ? process.env.HOME : null;
    const replacer = (k, v) => {
      if (typeof v === 'string') {
        let s = v;
        if (home && s.startsWith(home)) s = `~${s.slice(home.length)}`;
        if (s.length > 300) s = s.slice(0, 300) + '…';
        return s;
      }
      return v;
    };
    return JSON.stringify(args || {}, replacer);
  } catch { return String(args); }
}

function tauriDialog() {
  const dlg = getTauri().dialog;
  if (!dlg) throw new Error("Tauri dialog API not available");
  return dlg;
}

function tauriShell() {
  const sh = getTauri().shell;
  if (!sh) throw new Error("Tauri shell API not available");
  return sh;
}

function tauriEvents() {
  return getTauri().event;
}

const state = {
  scan: null,
  learn: null,
  export: null,
  logLevel: localStorage.getItem("rimloc.logLevel") || "info",
  locale: localStorage.getItem("rimloc.locale") || "auto",
  theme: localStorage.getItem("rimloc.theme") || "auto",
  progress: {},
};

function $(id) {
  return document.getElementById(id);
}

// Safe helpers to avoid null access on optional inputs
function val(id) {
  const el = $(id);
  return el ? (el.value || "").trim() : "";
}
function isChecked(id) {
  const el = $(id);
  return !!(el && el.checked);
}

function bindPersist(id, key, fallback = "") {
  const el = $(id);
  const saved = localStorage.getItem(key);
  if (saved !== null) {
    el.value = saved;
  } else if (fallback !== undefined && fallback !== null) {
    el.value = fallback;
  }
  const handler = () => {
    localStorage.setItem(key, el.value.trim());
  };
  el.addEventListener("change", handler);
  el.addEventListener("blur", handler);
  el.addEventListener("input", () => {
    if (!el.matches("textarea")) {
      localStorage.setItem(key, el.value.trim());
    }
  });
  return el;
}

function bindPersistTextArea(id, key) {
  const el = $(id);
  const saved = localStorage.getItem(key);
  if (saved !== null) {
    el.value = saved;
  }
  const handler = () => localStorage.setItem(key, el.value);
  el.addEventListener("change", handler);
  el.addEventListener("blur", handler);
  return el;
}

function showToast(message, isError = false) {
  const toast = $("toast");
  toast.textContent = message;
  toast.classList.remove("error", "show");
  if (isError) {
    toast.classList.add("error");
  }
  void toast.offsetWidth; // restart animation
  toast.classList.add("show");
  clearTimeout(showToast.timer);
  showToast.timer = setTimeout(() => toast.classList.remove("show"), 4000);
  debugLog(isError ? "error" : "info", message);
}

function showError(err) {
  console.error(err);
  showToast(formatError(err), true);
  updateStatus("", true);
}

function formatError(err) {
  if (!err) return "Unknown error";
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message || err.toString();
  if (typeof err === "object") {
    if (typeof err.message === "string") return err.message;
    if (typeof err.error === "string") return err.error;
  }
  try {
    return JSON.stringify(err);
  } catch {
    return String(err);
  }
}

function setBusy(isBusy, label = "Working…") {
  const overlay = $("overlay");
  const text = $("overlay-text");
  if (isBusy) {
    text.textContent = label;
    overlay.classList.remove("hidden");
    document.querySelectorAll("button").forEach((btn) => {
      btn.dataset.prevDisabled = btn.disabled ? "1" : "0";
      btn.disabled = true;
    });
  } else {
    overlay.classList.add("hidden");
    document.querySelectorAll("button").forEach((btn) => {
      if (btn.dataset.prevDisabled === "0") {
        btn.disabled = false;
      }
      delete btn.dataset.prevDisabled;
    });
  }
}

async function runAction(label, fn) {
  setBusy(true, label);
  try {
    debugLog("info", `${label}`);
    const result = await fn();
    return result;
  } catch (err) {
    showError(err);
    throw err;
  } finally {
    setBusy(false);
  }
}

function pickDirectory(targetId) {
  return async () => {
    debugLog("debug", `pickDirectory -> ${targetId}`);
    const current = $(targetId).value.trim() || null;
    try {
      // Try JS API first
      const selected = await tauriDialog().open({ directory: true, multiple: false, defaultPath: current || undefined });
      if (selected) {
        $(targetId).value = selected;
        $(targetId).dispatchEvent(new Event("change"));
        showToast(selected);
        return;
      }
    } catch (e) {
      debugLog("warn", `dialog.open failed: ${formatError(e)}`);
    }
    // Last-resort browser-only fallback: prompt for path (avoid backend blocking dialogs)
    try {
      const manual = window.prompt("Enter folder path:", current || "");
      if (manual) {
        $(targetId).value = manual;
        $(targetId).dispatchEvent(new Event("change"));
        showToast(manual);
        return;
      }
    } catch {}
  };
}

function pickFile(targetId, filters) {
  return async () => {
    try {
      const selected = await tauriDialog().open({ multiple: false, filters });
      if (selected) {
        $(targetId).value = selected;
        $(targetId).dispatchEvent(new Event("change"));
      }
    } catch (e) {
      showError(e);
    }
  };
}

function pickSave(targetId, options = {}) {
  return async () => {
    try {
      const selected = await tauriDialog().save(options);
      if (selected) {
        $(targetId).value = selected;
        $(targetId).dispatchEvent(new Event("change"));
      }
    } catch (e) {
      // Fallback to manual prompt when dialog API is not available
      debugLog("warn", `save dialog failed: ${formatError(e)}`);
      const current = $(targetId).value.trim();
      const manual = window.prompt("Enter file path:", current || (options.defaultPath || ""));
      if (manual) {
        $(targetId).value = manual;
        $(targetId).dispatchEvent(new Event("change"));
      } else {
        showError(e);
      }
    }
  };
}

function truncate(text, limit = 120) {
  if (text.length <= limit) return text;
  return `${text.slice(0, limit - 1)}…`;
}

function parseTmRoots(value) {
  return value
    .split(/\r?\n|,/)
    .map((line) => line.trim())
    .filter(Boolean);
}

function updateStatus(message, isError = false) {
  const status = $("status-text");
  status.textContent = message;
  status.classList.toggle("error", isError);
}

function renderScan(result) {
  state.scan = result;
  const summary = $("scan-summary");
  const tableBody = $("scan-table-body");
  tableBody.textContent = "";
  if (!result) {
    summary.textContent = "No scan performed yet.";
    return;
  }
  summary.textContent = `Found ${result.total} entries (${result.keyed} Keyed, ${result.defInjected ?? result.def_injected} DefInjected).`;
  const limit = 500;
  const rows = result.units.slice(0, limit);
  window._lastScanUnits = result.units;
  rows.forEach((unit) => {
    const tr = document.createElement("tr");
    const key = document.createElement("td");
    key.textContent = unit.key;
    const kind = document.createElement("td");
    kind.textContent = typeof unit.kind === "string" ? unit.kind : JSON.stringify(unit.kind);
    const source = document.createElement("td");
    const sample = (unit.source || "").replace(/\s+/g, " ").trim();
    source.textContent = truncate(sample, 160);
    const path = document.createElement("td");
    path.textContent = unit.path;
    const line = document.createElement("td");
    line.textContent = unit.line != null ? String(unit.line) : "";
    tr.append(key, kind, source, path, line);
    tableBody.appendChild(tr);
  });
  if (result.units.length > limit) {
    const tr = document.createElement("tr");
    const td = document.createElement("td");
    td.colSpan = 5;
    td.textContent = `Showing first ${limit} entries of ${result.units.length}.`;
    tr.appendChild(td);
    tableBody.appendChild(tr);
  }
  updateStatus(`Scan complete – ${result.total} entries`);
  debugLog("info", `Scan complete: ${result.total} entries (${result.keyed} keyed, ${result.defInjected ?? result.def_injected} definj)`);
  // refresh preview panel
  try { renderPreview(); } catch (_) {}
}

function makePathRow(label, path) {
  const wrapper = document.createElement("div");
  wrapper.className = "paths-row";
  const text = document.createElement("span");
  text.textContent = `${label}: ${path}`;
  const btn = document.createElement("button");
  btn.type = "button";
  btn.textContent = "Open";
  btn.addEventListener("click", () => openPath(path));
  wrapper.append(text, btn);
  return wrapper;
}

function renderLearn(result) {
  state.learn = result;
  const container = $("learn-result");
  container.textContent = "";
  if (!result) {
    container.textContent = "No learn run yet.";
    $("learn-open-out").disabled = true;
    return;
  }
  const summary = document.createElement("p");
  summary.textContent = `Accepted ${result.accepted} of ${result.candidates} candidates.`;
  container.appendChild(summary);
  const paths = document.createElement("div");
  paths.className = "paths";
  paths.appendChild(makePathRow("Missing keys", result.missingPath || result.missing_path));
  paths.appendChild(makePathRow("Suggested XML", result.suggestedPath || result.suggested_path));
  paths.appendChild(makePathRow("Learned dataset", result.learnedPath || result.learned_path));
  container.appendChild(paths);
  $("learn-open-out").disabled = false;
  debugLog("info", `Learn completed. Accepted ${result.accepted} / ${result.candidates}`);
}

function renderExport(result) {
  state.export = result;
  const container = $("export-result");
  container.textContent = "";
  if (!result) {
    container.textContent = "No export performed yet.";
    $("export-open-file").disabled = true;
    return;
  }
  const summary = document.createElement("p");
  summary.textContent = `Saved ${result.outPo || result.out_po}. Entries: ${result.total}, TM filled: ${result.tmFilled ?? result.tm_filled} (${result.tmCoveragePct ?? result.tm_coverage_pct}% ).`;
  container.appendChild(summary);
  if (result.warning) {
    const warn = document.createElement("p");
    warn.textContent = result.warning;
    warn.className = "warning";
    container.appendChild(warn);
    debugLog("warn", result.warning);
  }
  $("export-open-file").disabled = false;
}

async function handleScan(saveMode) {
  try { console.log('Clicked Scan', saveMode||'run'); } catch {}
  debugLog('info', `scan clicked (${saveMode||'run'})`, true);
  const root = val("mod-root");
  if (!root) {
    showToast("Select mod root first", true);
    return;
  }
  const payload = {
    root,
    game_version: val("game-version") || null,
    lang: val("target-lang") || null,
    include_all_versions: isChecked("scan-all-versions"),
    source_lang: val("source-lang") || null,
    source_lang_dir: null,
    defs_root: val("scan-defs-root") || null,
    extra_fields: (val("scan-extra-fields") || "").split(',').map(s => s.trim()).filter(Boolean),
    defs_dicts: (($("scan-defs-dicts")?.value||"").split(/\r?\n/).map(s=>s.trim()).filter(Boolean)),
    type_schema: val("scan-type-schema") || null,
    keyed_nested: isChecked("scan-keyed-nested"),
    no_inherit: isChecked("scan-no-inherit"),
    with_plugins: isChecked("scan-with-plugins"),
  };
  debugLog("debug", `scan payload: ${JSON.stringify(payload)}`);
  if (saveMode === "json") {
    const path = await tauriDialog().save({
      defaultPath: `${root.replace(/\\/g, "/")}/_learn/scan.json`,
    });
    if (!path) return;
    payload.out_json = path;
    await runAction("Saving JSON…", () => tauriInvoke("scan_mod", { request: payload }));
    showToast(`Saved scan JSON to ${path}`);
    return;
  }
  if (saveMode === "csv") {
    const path = await tauriDialog().save({
      defaultPath: `${root.replace(/\\/g, "/")}/_learn/scan.csv`,
    });
    if (!path) return;
    payload.out_csv = path;
    await runAction("Saving CSV…", () => tauriInvoke("scan_mod", { request: payload }));
    showToast(`Saved scan CSV to ${path}`);
    return;
  }
  const result = await runAction("Scanning…", () => tauriInvoke("scan_mod", { request: payload }));
  try { console.log('Scan result', result); } catch {}
  debugLog('info', `scan done: total=${result.total}`);
  renderScan(result);
  showToast(`Found ${result.total} entries`);
}

async function handleLearn() {
  try { console.log('Clicked Learn'); } catch {}
  debugLog('info', 'learn clicked', true);
  const root = val("mod-root");
  if (!root) {
    showToast("Select mod root first", true);
    return;
  }
  const outDir = val("learn-out");
  const langDir = val("learn-lang-dir");
  const thresholdStr = val("learn-threshold");
  const threshold = thresholdStr ? Number(thresholdStr) : null;
  const payload = {
    root,
    out_dir: outDir || null,
    lang_dir: langDir || null,
    game_version: val("game-version") || null,
    threshold: threshold,
  };
  debugLog("debug", `learn payload: ${JSON.stringify(payload)}`);
  const result = await runAction("Learning DefInjected…", () => tauriInvoke("learn_defs", { request: payload }));
  try { console.log('Learn result', result); } catch {}
  debugLog('info', `learn done: accepted=${result.accepted}/${result.candidates}`);
  renderLearn(result);
  showToast(tr("toast_learned"));
  updateStatus("Learn completed");
}

async function handleExport() {
  try { console.log('Clicked Export'); } catch {}
  debugLog('info', 'export clicked', true);
  const root = val("mod-root");
  if (!root) {
    showToast("Select mod root first", true);
    return;
  }
  let outPo = val("po-output");
  if (!outPo) {
    outPo = `${root.replace(/\\/g,'/')}/_learn/translation.po`;
    $("po-output").value = outPo;
  }
  const tmRoots = parseTmRoots(val("tm-roots"));
  const pot = isChecked("export-pot");
  const include_all_versions = isChecked("export-all-versions");
  const payload = {
    root,
    out_po: outPo,
    lang: val("target-lang") || null,
    source_lang: val("source-lang") || null,
    source_lang_dir: val("export-source-lang-dir") || null,
    tm_roots: tmRoots.length ? tmRoots : null,
    game_version: val("game-version") || null,
    pot,
    include_all_versions,
  };
  debugLog("debug", `export payload: ${JSON.stringify(payload)}`);
  const result = await runAction("Exporting PO…", () => tauriInvoke("export_po", { request: payload }));
  try { console.log('Export result', result); } catch {}
  debugLog('info', `export done: total=${result.total}`);
  renderExport(result);
  showToast("PO exported successfully");
  updateStatus("Export finished");
}

async function handleValidate() {
  try { console.log('Clicked Validate'); } catch {}
  debugLog('info', 'validate clicked', true);
  const root = val("mod-root");
  if (!root) return showToast("Select mod root first", true);
  const payload = {
    root,
    game_version: val("game-version") || null,
    source_lang: val("validate-source-lang") || null,
    source_lang_dir: val("validate-source-lang-dir") || null,
    defs_root: val("validate-defs-root") || null,
    extra_fields: (val("validate-extra-fields") || "").split(',').map(s => s.trim()).filter(Boolean),
    defs_dicts: (($("validate-defs-dicts")?.value||"").split(/\r?\n/).map(s=>s.trim()).filter(Boolean)),
    defs_type_schema: val("validate-type-schema") || null,
    include_all_versions: isChecked("validate-all-versions"),
    compare_placeholders: isChecked("validate-compare-ph"),
    target_lang: val("validate-target-lang") || null,
    target_lang_dir: val("validate-target-lang-dir") || null,
  };
  debugLog("debug", `validate payload: ${JSON.stringify(payload)}`);
  const result = await runAction("Validating…", () => tauriInvoke("validate_mod", { request: payload }));
  try { console.log('Validate result', result); } catch {}
  debugLog('info', `validate done: total=${result.total}`);
  renderValidate(result);
  showToast(`Validation: ${result.total} messages`);
}

function renderValidate(result) {
  const box = $("validate-result");
  box.textContent = "";
  if (!result) { box.textContent = tr("validate_empty"); return; }
  const summary = document.createElement("p");
  summary.textContent = `Total: ${result.total}, errors: ${result.errors}, warnings: ${result.warnings}, info: ${result.infos}`;
  box.appendChild(summary);
  const pre = document.createElement("pre");
  pre.className = 'scroll-code';
  const lines = (result.messages || []).slice(0, 500).map(m => `${(m.kind||'').toUpperCase()} ${m.path}${m.line?':'+m.line:''}: ${m.key} – ${m.message}`);
  pre.textContent = lines.join("\n");
  box.appendChild(pre);
}

async function handleHealth() {
  try { console.log('Clicked Health'); } catch {}
  debugLog('info', 'health clicked', true);
  const root = val("mod-root");
  if (!root) return showToast("Select mod root first", true);
  const payload = {
    root,
    game_version: val("game-version") || null,
    lang_dir: val("health-lang-dir") || null,
    strict: isChecked("health-strict"),
    only: (val("health-only") || "").split(',').map(s => s.trim()).filter(Boolean),
    except: (val("health-except") || "").split(',').map(s => s.trim()).filter(Boolean),
  };
  const result = await runAction("XML Health…", () => tauriInvoke("xml_health", { request: payload }));
  try { console.log('Health result', result); } catch {}
  debugLog('info', `health done: checked=${result.checked} issues=${result.issues?.length||0}`);
  renderHealth(result);
  showToast(`Checked ${result.checked}, issues: ${result.issues.length}`);
}

function renderHealth(result) {
  const box = $("health-result");
  box.textContent = "";
  if (!result) { box.textContent = tr("health_empty"); return; }
  const summary = document.createElement("p");
  summary.textContent = `Checked: ${result.checked}, issues: ${result.issues.length}`;
  box.appendChild(summary);
  const pre = document.createElement("pre");
  pre.className = 'scroll-code';
  pre.textContent = (result.issues || []).slice(0, 500).map(i => `${i.path}${i.line?':'+i.line:''}: ${i.kind} – ${i.message}`).join("\n");
  box.appendChild(pre);
}

async function handleImport() {
  try { console.log('Clicked Import'); } catch {}
  debugLog('info', 'import clicked', true);
  const root = val("mod-root");
  if (!root) return showToast("Select mod root first", true);
  const po_path = val("import-po");
  if (!po_path) return showToast("Select PO file", true);
  const payload = {
    root,
    po_path,
    game_version: val("game-version") || null,
    lang_dir: val("import-lang-dir") || null,
    keep_empty: isChecked("import-keep-empty"),
    backup: isChecked("import-backup"),
    single_file: isChecked("import-single-file"),
    incremental: isChecked("import-incremental"),
    only_diff: isChecked("import-only-diff"),
    report: isChecked("import-report"),
    dry_run: isChecked("import-dry-run"),
  };
  debugLog("debug", `import payload: ${JSON.stringify(payload)}`);
  const result = await runAction("Importing PO…", () => tauriInvoke("import_po", { request: payload }));
  try { console.log('Import result', result); } catch {}
  debugLog('info', `import done: created=${result.created} updated=${result.updated}`);
  const box = $("import-result");
  box.textContent = `Created: ${result.created}, Updated: ${result.updated}, Skipped: ${result.skipped}, Keys: ${result.keys}`;
}

async function handleBuild() {
  try { console.log('Clicked Build'); } catch {}
  debugLog('info', 'build clicked', true);
  const po_path = val("build-po");
  const out_mod = val("build-out");
  const lang_dir = val("build-lang-dir") || "Russian";
  const name = val("build-name") || "RimLoc Translation";
  const package_id = val("build-package") || "your.name.rimloc";
  const rw_version = val("build-rw") || "1.5";
  const dedupe = isChecked("build-dedupe");
  const dry_run = isChecked("build-dry");
  const from_root = val("build-from-root");
  const from_game_versions = (val("build-from-versions") || "").split(',').map(s => s.trim()).filter(Boolean);
  if (!out_mod) return showToast("Select output folder", true);
  if (!po_path && !from_root) return showToast("Select PO or From root", true);
  const payload = { po_path, out_mod, lang_dir, name, package_id, rw_version, dedupe, dry_run, from_root: from_root || null, from_game_versions: from_game_versions.length ? from_game_versions : null };
  debugLog("debug", `build payload: ${JSON.stringify(payload)}`);
  const result = await runAction("Building mod…", () => tauriInvoke("build_mod", { request: payload }));
  try { console.log('Build result', result); } catch {}
  debugLog('info', `build done: files=${result.files}`);
  const box = $("build-result");
  if (box) box.textContent = `Built to ${result.outMod || result.out_mod}. Files: ${result.files}, Keys: ${result.totalKeys || result.total_keys || 0}`;
  showToast("Build complete");
}

async function handleDiff() {
  try { console.log('Clicked Diff'); } catch {}
  debugLog('info', 'diff clicked', true);
  const root = val("mod-root");
  if (!root) return showToast("Select mod root first", true);
  const payload = {
    root,
    game_version: val("game-version") || null,
    source_lang_dir: val("diff-source-lang-dir") || "English",
    target_lang_dir: val("diff-target-lang-dir") || "Russian",
    defs_root: val("diff-defs-root") || null,
    baseline_po: val("diff-po") || null,
    defs_dicts: (($("diff-defs-dicts")?.value||"").split(/\r?\n/).map(s=>s.trim()).filter(Boolean)),
    type_schema: val("diff-type-schema") || null,
    extra_fields: (val("diff-extra-fields") || "").split(',').map(s=>s.trim()).filter(Boolean),
  };
  const res = await runAction("Diff XML…", () => tauriInvoke("diff_xml_cmd", { request: payload }));
  try { console.log('Diff result', res); } catch {}
  debugLog('info', `diff done: only_in_mod=${res.only_in_mod?.length||0} only_in_translation=${res.only_in_translation?.length||0} changed=${res.changed?.length||0}`);
  const box = $("diff-result");
  box.textContent = `Only in mod: ${res.only_in_mod.length}, Only in translation: ${res.only_in_translation.length}, Changed: ${res.changed.length}`;
  const pre = document.createElement("pre");
  pre.textContent = [
    "-- Only in mod --",
    ...res.only_in_mod.slice(0, 100),
    "",
    "-- Only in translation --",
    ...res.only_in_translation.slice(0, 100),
    "",
    "-- Changed --",
    ...res.changed.slice(0, 100).map((pair) => Array.isArray(pair) ? pair[0] : (pair.key || JSON.stringify(pair)))
  ].join("\n");
  box.appendChild(document.createElement("br"));
  box.appendChild(pre);
}

async function handleMorph() {
  try { console.log('Clicked Morph'); } catch {}
  const root = val("mod-root"); if (!root) return showToast("Select mod root first", true);
  const payload = {
    root,
    target_lang_dir: val("morph-target-lang-dir") || "Russian",
    provider: $("morph-provider")?.value || "dummy",
    filter_key_regex: val("morph-filter") || null,
    limit: (val("morph-limit")||"") ? Number(val("morph-limit")) : null,
    timeout_ms: (val("morph-timeout")||"") ? Number(val("morph-timeout")) : null,
    cache_size: (val("morph-cache")||"") ? Number(val("morph-cache")) : null,
    pymorphy_url: val("morph-pym-url") || null,
    morpher_token: val("morph-token") || null,
  };
  const res = await runAction("Generating morph…", () => tauriInvoke("morph_cmd", { request: payload }));
  $("morph-result").textContent = `Processed: ${res.processed}, Lang: ${res.lang}${res.warnNoMorpher?' (no MORPHER_TOKEN)':''}${res.warnNoPymorphy?' (no Pymorphy URL)':''}`;
}

async function handleLearnKeyed() {
  try { console.log('Clicked Learn Keyed'); } catch {}
  const root = val("mod-root"); if (!root) return showToast("Select mod root first", true);
  const dicts = (($("learn-keyed-dicts")?.value||"").split(/\r?\n/).map(s=>s.trim()).filter(Boolean));
  const blacklist = (($("learn-keyed-blacklist")?.value||"").split(/\r?\n/).map(s=>s.trim()).filter(Boolean));
  const exclude = (($("learn-keyed-exclude")?.value||"").split(/\r?\n/).map(s=>s.trim()).filter(Boolean));
  const payload = {
    root,
    source_lang_dir: val("learn-keyed-source") || "English",
    target_lang_dir: val("learn-keyed-target") || "Russian",
    dict_files: dicts.length?dicts:null,
    min_len: (val("learn-keyed-minlen")||"")?Number(val("learn-keyed-minlen")):null,
    blacklist: blacklist.length?blacklist:null,
    must_contain_letter: isChecked("learn-keyed-must-letter"),
    exclude_substr: exclude.length?exclude:null,
    threshold: (val("learn-keyed-threshold")||"")?Number(val("learn-keyed-threshold")):null,
    out_dir: val("learn-keyed-out") || "_learn",
    from_defs_special: isChecked("learn-keyed-from-defs"),
  };
  const res = await runAction("Learn Keyed…", () => tauriInvoke("learn_keyed_cmd", { request: payload }));
  $("learn-keyed-result").textContent = `Processed: ${res.processed}, Suggested: ${res.suggested}, Missing: ${res.missing}`;
}

async function handleDumpSchemas() {
  try { console.log('Clicked Dump Schemas'); } catch {}
  const out_dir = val("schemas-out") || "./docs/assets/schemas";
  const saved = await runAction("Dumping schemas…", () => tauriInvoke("dump_schemas", { req: { out_dir: out_dir } }));
  $("schemas-result").textContent = `Saved to: ${saved}`;
}

async function handleLangUpdate() {
  try { console.log('Clicked Lang Update'); } catch {}
  debugLog('info', 'lang_update clicked', true);
  const root = val("lang-update-game-root") || val("mod-root");
  if (!root) return showToast("Select game root first", true);
  const payload = {
    root,
    repo: val("lang-update-repo") || "Ludeon/RimWorld",
    branch: val("lang-update-branch") || null,
    source_lang_dir: val("lang-update-source") || "English",
    target_lang_dir: val("lang-update-target") || "Russian",
    dry_run: isChecked("lang-update-dry"),
    backup: isChecked("lang-update-backup"),
  };
  const res = await runAction("Lang update…", () => tauriInvoke("lang_update_cmd", { request: payload }));
  try { console.log('Lang update result', res); } catch {}
  debugLog('info', `lang_update done: files=${res.files} bytes=${res.bytes}`);
  $("lang-update-result").textContent = `Files: ${res.files}, Bytes: ${res.bytes}, Out: ${res.outDir || res.out_dir}`;
}

async function handleAnnotate(dry) {
  try { console.log('Clicked Annotate', { dry }); } catch {}
  debugLog('info', `annotate clicked (dry=${!!dry})`, true);
  const root = val("mod-root");
  if (!root) return showToast("Select mod root first", true);
  const payload = {
    root,
    source_lang_dir: val("annotate-source") || "English",
    target_lang_dir: val("annotate-target") || "Russian",
    comment_prefix: val("annotate-prefix") || "//",
    strip: isChecked("annotate-strip"),
    dry_run: !!dry,
    backup: isChecked("annotate-backup"),
  };
  const res = await runAction(dry ? "Annotate preview…" : "Annotate apply…", () => tauriInvoke("annotate_cmd", { request: payload }));
  try { console.log('Annotate result', res); } catch {}
  debugLog('info', `annotate done: processed=${res.processed} annotated=${res.annotated}`);
  $("annotate-result").textContent = `Processed: ${res.processed}, Annotated: ${res.annotated}`;
}

async function handleInit() {
  try { console.log('Clicked Init'); } catch {}
  const root = val("mod-root");
  if (!root) return showToast("Select mod root first", true);
  const payload = {
    root,
    source_lang_dir: val("init-source") || "English",
    target_lang_dir: val("init-target") || "Russian",
    overwrite: isChecked("init-overwrite"),
    dry_run: isChecked("init-dry"),
  };
  const res = await runAction("Init language…", () => tauriInvoke("init_lang_cmd", { request: payload }));
  try { console.log('Init result', res); } catch {}
  $("init-result").textContent = `Files: ${res.files}, Language: ${res.outLanguage || res.out_language}`;
}

async function openPath(path) {
  if (!path) return;
  const s = String(path || '');
  // Always use backend command which supports files and URLs across OSes
  try { await tauriInvoke("open_path", { path: s }); }
  catch (e) { showError(e); }
}

function initEventHandlers() {
  $("scan-run").addEventListener("click", () => handleScan());
  $("scan-save-json").addEventListener("click", () => handleScan("json"));
  $("scan-save-csv").addEventListener("click", () => handleScan("csv"));
  $("learn-run").addEventListener("click", handleLearn);
  $("learn-open-out").addEventListener("click", () => {
    const path = state.learn?.outDir || state.learn?.out_dir;
    openPath(path);
  });
  $("export-run").addEventListener("click", handleExport);
  $("export-open-file").addEventListener("click", () => {
    const path = state.export?.outPo || state.export?.out_po;
    openPath(path);
  });

  document.querySelector('[data-action="pick-root"]').addEventListener("click", pickDirectory("mod-root"));
  const pickScanDefs = document.querySelector('[data-action="pick-scan-defs"]');
  if (pickScanDefs) pickScanDefs.addEventListener("click", pickDirectory("scan-defs-root"));
  const pickScanSchema = document.querySelector('[data-action="pick-scan-schema"]');
  if (pickScanSchema) pickScanSchema.addEventListener("click", () => pickFile("scan-type-schema", [{ name: "JSON", extensions: ["json"] }])());
  document.querySelector('[data-action="pick-learn-out"]').addEventListener("click", pickDirectory("learn-out"));
  document.querySelector('[data-action="pick-po-output"]').addEventListener(
    "click",
    pickSave("po-output", { defaultPath: "translation.po" })
  );

  // Validate
  $("validate-run").addEventListener("click", handleValidate);
  const pickDefs = document.querySelector('[data-action="pick-validate-defs"]');
  if (pickDefs) pickDefs.addEventListener("click", pickDirectory("validate-defs-root"));
  const pickValSchema = document.querySelector('[data-action="pick-validate-schema"]');
  if (pickValSchema) pickValSchema.addEventListener("click", () => pickFile("validate-type-schema", [{ name: "JSON", extensions: ["json"] }])());

  // Health
  $("health-run").addEventListener("click", handleHealth);
  const healthSave = $("health-save"); if (healthSave) healthSave.addEventListener("click", async () => {
    const root = val("mod-root"); if (!root) return showToast("Select mod root first", true);
    const path = await tauriDialog().save({ defaultPath: `${root.replace(/\\/g,'/')}/_learn/health.json` });
    if (!path) return; await runAction("Saving health…", () => tauriInvoke("xml_health", { request: { root, game_version: val("game-version") || null, lang_dir: val("health-lang-dir") || null, out_json: path } }));
    showToast(`Saved: ${path}`);
  });

  // Import
  $("import-run").addEventListener("click", handleImport);
  const pickImportPo = document.querySelector('[data-action="pick-import-po"]');
  if (pickImportPo) pickImportPo.addEventListener("click", () => pickFile("import-po", [{ name: "PO", extensions: ["po"] }])());
  const pickImportOutXml = document.querySelector('[data-action="pick-import-out-xml"]');
  if (pickImportOutXml) pickImportOutXml.addEventListener("click", () => pickSave("import-out-xml", { defaultPath: `${(val('mod-root')||'').replace(/\\/g,'/')}/_learn/_Imported.xml` })());
  const importSingle = $("import-single-file");
  if (importSingle) importSingle.addEventListener("change", () => {
    const wrap = $("import-out-xml-wrap");
    if (wrap) wrap.style.display = importSingle.checked ? '' : 'none';
    if (importSingle.checked && !val('import-out-xml')) {
      const root = val('mod-root');
      if (root) $("import-out-xml").value = `${root.replace(/\\/g,'/')}/_learn/_Imported.xml`;
    }
  });

  // Build
  const buildRun = $("build-run");
  if (buildRun) buildRun.addEventListener("click", handleBuild);
  const pickBuildPo = document.querySelector('[data-action="pick-build-po"]');
  if (pickBuildPo) pickBuildPo.addEventListener("click", () => pickFile("build-po", [{ name: "PO", extensions: ["po"] }])());
  const pickBuildOut = document.querySelector('[data-action="pick-build-out"]');
  if (pickBuildOut) pickBuildOut.addEventListener("click", pickDirectory("build-out"));
  const pickBuildFrom = document.querySelector('[data-action="pick-build-from-root"]');
  if (pickBuildFrom) pickBuildFrom.addEventListener("click", pickDirectory("build-from-root"));

  // Diff
  const diffRun = $("diff-run");
  if (diffRun) diffRun.addEventListener("click", handleDiff);
  const pickDiffDefs = document.querySelector('[data-action="pick-diff-defs"]');
  if (pickDiffDefs) pickDiffDefs.addEventListener("click", pickDirectory("diff-defs-root"));
  const pickDiffPo = document.querySelector('[data-action="pick-diff-po"]');
  if (pickDiffPo) pickDiffPo.addEventListener("click", () => pickFile("diff-po", [{ name: "PO", extensions: ["po"] }])());
  const pickDiffSchema = document.querySelector('[data-action="pick-diff-schema"]');
  if (pickDiffSchema) pickDiffSchema.addEventListener("click", () => pickFile("diff-type-schema", [{ name: "JSON", extensions: ["json"] }])());

  // Morph
  const morphRun = $("morph-run"); if (morphRun) morphRun.addEventListener("click", handleMorph);

  // Learn Keyed
  const lkRun = $("learn-keyed-run"); if (lkRun) lkRun.addEventListener("click", handleLearnKeyed);
  const pickLkOut = document.querySelector('[data-action="pick-learn-keyed-out"]');
  if (pickLkOut) pickLkOut.addEventListener("click", pickDirectory("learn-keyed-out"));

  // Schemas
  const dumpSchemas = $("schemas-dump"); if (dumpSchemas) dumpSchemas.addEventListener("click", handleDumpSchemas);
  const pickSchemasOut = document.querySelector('[data-action="pick-schemas-out"]');
  if (pickSchemasOut) pickSchemasOut.addEventListener("click", pickDirectory("schemas-out"));

  // Lang update
  const luRun = $("lang-update-run");
  if (luRun) luRun.addEventListener("click", handleLangUpdate);
  const pickLuRoot = document.querySelector('[data-action="pick-lang-update-root"]');
  if (pickLuRoot) pickLuRoot.addEventListener("click", pickDirectory("lang-update-game-root"));

  // Annotate
  const annPrev = $("annotate-preview");
  if (annPrev) annPrev.addEventListener("click", () => handleAnnotate(true));
  const annApply = $("annotate-apply");
  if (annApply) annApply.addEventListener("click", () => handleAnnotate(false));

  // Init
  const initRun = $("init-run");
  if (initRun) initRun.addEventListener("click", handleInit);
}

function initPersistence() {
  bindPersist("mod-root", "rimloc.modRoot");
  bindPersist("game-version", "rimloc.gameVersion");
  bindPersist("target-lang", "rimloc.targetLang", "ru");
  bindPersist("source-lang", "rimloc.sourceLang");
  bindPersist("learn-out", "rimloc.learnOut", "_learn");
  bindPersist("learn-lang-dir", "rimloc.learnLangDir", "English");
  bindPersist("po-output", "rimloc.poOutput", "_learn/translation.po");
  bindPersistTextArea("tm-roots", "rimloc.tmRoots");
  // Scan options
  const scanAll = $("scan-all-versions");
  if (scanAll) {
    scanAll.checked = localStorage.getItem("rimloc.scanAllVersions") === "1";
    scanAll.addEventListener("change", () => localStorage.setItem("rimloc.scanAllVersions", scanAll.checked ? "1" : "0"));
  }
  bindPersist("scan-defs-root", "rimloc.scanDefsRoot");
  bindPersist("scan-extra-fields", "rimloc.scanExtraFields");
  bindPersistTextArea("scan-defs-dicts", "rimloc.scanDefsDicts");
  bindPersist("scan-type-schema", "rimloc.scanTypeSchema");
  ["scan-keyed-nested","scan-no-inherit","scan-with-plugins"].forEach(id => {
    const el = $(id);
    if (!el) return; const key = `rimloc.${id}`;
    el.checked = localStorage.getItem(key) === "1";
    el.addEventListener("change", () => localStorage.setItem(key, el.checked?"1":"0"));
  });
  // Export options
  bindPersist("export-source-lang-dir", "rimloc.exportSourceLangDir", "English");
  const expPot = $("export-pot");
  if (expPot) {
    expPot.checked = localStorage.getItem("rimloc.exportPOT") === "1";
    expPot.addEventListener("change", () => localStorage.setItem("rimloc.exportPOT", expPot.checked ? "1" : "0"));
  }
  const expAll = $("export-all-versions");
  if (expAll) {
    expAll.checked = localStorage.getItem("rimloc.exportAllVersions") === "1";
    expAll.addEventListener("change", () => localStorage.setItem("rimloc.exportAllVersions", expAll.checked ? "1" : "0"));
  }
  bindPersist("learn-threshold", "rimloc.learnThreshold", "0.8");
  // Validate options
  bindPersist("validate-source-lang", "rimloc.validateSourceLang");
  bindPersist("validate-source-lang-dir", "rimloc.validateSourceLangDir", "English");
  bindPersist("validate-defs-root", "rimloc.validateDefsRoot");
  bindPersist("validate-extra-fields", "rimloc.validateExtraFields");
  bindPersistTextArea("validate-defs-dicts", "rimloc.validateDefsDicts");
  bindPersist("validate-type-schema", "rimloc.validateTypeSchema");
  const valAll = $("validate-all-versions"); if (valAll) { valAll.checked = localStorage.getItem("rimloc.validateAllVersions") === "1"; valAll.addEventListener("change", () => localStorage.setItem("rimloc.validateAllVersions", valAll.checked?"1":"0")); }
  const valCmp = $("validate-compare-ph"); if (valCmp) { valCmp.checked = localStorage.getItem("rimloc.validateComparePH") === "1"; valCmp.addEventListener("change", () => localStorage.setItem("rimloc.validateComparePH", valCmp.checked?"1":"0")); }
  bindPersist("validate-target-lang", "rimloc.validateTargetLang", "ru");
  bindPersist("validate-target-lang-dir", "rimloc.validateTargetLangDir", "Russian");
  // Health options
  bindPersist("health-lang-dir", "rimloc.healthLangDir", "English");
  const hStrict = $("health-strict"); if (hStrict) { hStrict.checked = localStorage.getItem("rimloc.healthStrict") === "1"; hStrict.addEventListener("change", () => localStorage.setItem("rimloc.healthStrict", hStrict.checked?"1":"0")); }
  bindPersist("health-only", "rimloc.healthOnly");
  bindPersist("health-except", "rimloc.healthExcept");
  // Import options
  bindPersist("import-po", "rimloc.importPo");
  bindPersist("import-lang-dir", "rimloc.importLangDir", "Russian");
  const flags = ["import-single-file","import-incremental","import-keep-empty","import-backup","import-only-diff","import-report","import-dry-run"];
  flags.forEach(id => {
    const el = $(id);
    if (!el) return;
    const key = `rimloc.${id}`;
    el.checked = localStorage.getItem(key) === "1";
    el.addEventListener("change", () => localStorage.setItem(key, el.checked ? "1" : "0"));
  });

  // Build options
  bindPersist("build-po", "rimloc.buildPo");
  bindPersist("build-out", "rimloc.buildOut");
  bindPersist("build-lang-dir", "rimloc.buildLangDir", "Russian");
  bindPersist("build-name", "rimloc.buildName", "RimLoc Translation");
  bindPersist("build-package", "rimloc.buildPackage", "your.name.rimloc");
  bindPersist("build-rw", "rimloc.buildRW", "1.5");
  bindPersist("build-from-root", "rimloc.buildFromRoot");
  bindPersist("build-from-versions", "rimloc.buildFromVersions");
  const buildDedupe = $("build-dedupe");
  if (buildDedupe) {
    buildDedupe.checked = localStorage.getItem("rimloc.buildDedupe") === "1";
    buildDedupe.addEventListener("change", () => localStorage.setItem("rimloc.buildDedupe", buildDedupe.checked ? "1" : "0"));
  }
  const buildDry = $("build-dry"); if (buildDry) { buildDry.checked = localStorage.getItem("rimloc.buildDry") === "1"; buildDry.addEventListener("change", () => localStorage.setItem("rimloc.buildDry", buildDry.checked?"1":"0")); }

  // Diff options
  bindPersist("diff-source-lang-dir", "rimloc.diffSource", "English");
  bindPersist("diff-target-lang-dir", "rimloc.diffTarget", "Russian");
  bindPersist("diff-defs-root", "rimloc.diffDefs");
  bindPersist("diff-po", "rimloc.diffPo");
  bindPersistTextArea("diff-defs-dicts", "rimloc.diffDefsDicts");
  bindPersist("diff-type-schema", "rimloc.diffTypeSchema");
  bindPersist("diff-extra-fields", "rimloc.diffExtraFields");

  // Lang update options
  bindPersist("lang-update-repo", "rimloc.luRepo", "Ludeon/RimWorld");
  bindPersist("lang-update-branch", "rimloc.luBranch", "master");
  bindPersist("lang-update-source", "rimloc.luSource", "English");
  bindPersist("lang-update-target", "rimloc.luTarget", "Russian");
  bindPersist("lang-update-game-root", "rimloc.luGameRoot");

  // Morph persist
  bindPersist("morph-target-lang-dir", "rimloc.morphTarget", "Russian");
  const morphProvider = $("morph-provider"); if (morphProvider) { morphProvider.value = localStorage.getItem("rimloc.morphProvider") || "dummy"; morphProvider.addEventListener("change", () => localStorage.setItem("rimloc.morphProvider", morphProvider.value)); }
  bindPersist("morph-filter", "rimloc.morphFilter");
  bindPersist("morph-limit", "rimloc.morphLimit");
  bindPersist("morph-timeout", "rimloc.morphTimeout", "1500");
  bindPersist("morph-cache", "rimloc.morphCache", "1024");
  bindPersist("morph-pym-url", "rimloc.morphPymUrl");
  bindPersist("morph-token", "rimloc.morphToken");

  // Learn Keyed persist
  bindPersist("learn-keyed-out", "rimloc.lkOut", "_learn");
  bindPersist("learn-keyed-source", "rimloc.lkSource", "English");
  bindPersist("learn-keyed-target", "rimloc.lkTarget", "Russian");
  bindPersist("learn-keyed-threshold", "rimloc.lkThreshold", "0.8");
  bindPersistTextArea("learn-keyed-dicts", "rimloc.lkDicts");
  bindPersistTextArea("learn-keyed-blacklist", "rimloc.lkBlacklist");
  bindPersistTextArea("learn-keyed-exclude", "rimloc.lkExclude");
  bindPersist("learn-keyed-minlen", "rimloc.lkMinLen", "1");
  ["learn-keyed-must-letter","learn-keyed-from-defs"].forEach(id => { const el=$(id); if (!el) return; const k=`rimloc.${id}`; el.checked = localStorage.getItem(k)==="1"; el.addEventListener("change", ()=>localStorage.setItem(k, el.checked?"1":"0")); });

  // Schemas persist
  bindPersist("schemas-out", "rimloc.schemasOut", "./docs/assets/schemas");
  ["lang-update-dry","lang-update-backup"].forEach(id => {
    const el = $(id);
    if (!el) return;
    const key = `rimloc.${id}`;
    el.checked = localStorage.getItem(key) === "1";
    el.addEventListener("change", () => localStorage.setItem(key, el.checked ? "1" : "0"));
  });

  // Annotate options
  bindPersist("annotate-source", "rimloc.annSource", "English");
  bindPersist("annotate-target", "rimloc.annTarget", "Russian");
  bindPersist("annotate-prefix", "rimloc.annPrefix", "//");
  ["annotate-strip","annotate-backup"].forEach(id => {
    const el = $(id);
    if (!el) return;
    const key = `rimloc.${id}`;
    el.checked = localStorage.getItem(key) === "1";
    el.addEventListener("change", () => localStorage.setItem(key, el.checked ? "1" : "0"));
  });

  // Init options
  bindPersist("init-source", "rimloc.initSource", "English");
  bindPersist("init-target", "rimloc.initTarget", "Russian");
  ["init-overwrite","init-dry"].forEach(id => {
    const el = $(id);
    if (!el) return;
    const key = `rimloc.${id}`;
    el.checked = localStorage.getItem(key) === "1";
    el.addEventListener("change", () => localStorage.setItem(key, el.checked ? "1" : "0"));
  });
}

async function fetchAppVersion() {
  try {
    const info = await tauriInvoke("get_app_info");
    if (info?.version) {
      $("app-version").textContent = `v${info.version}`;
    }
  } catch (err) {
    console.warn("Unable to read app version", err);
  }
  const ev = tauriEvents();
  if (ev?.listen) {
    ev
      .listen("app-info", ({ payload }) => {
        if (payload?.version) {
          $("app-version").textContent = `v${payload.version}`;
        }
      })
      .catch(() => {});
    ev
      .listen("log", ({ payload }) => {
        if (!payload) return;
        // Log backend messages to UI only to avoid echo loop
        debugLog(payload.level || "info", payload.message || String(payload), true);
      })
      .catch(() => {});

    ev
      .listen("progress", ({ payload }) => {
        if (!payload) return;
        const { action, step, message, pct } = payload;
        const msg = `[${action}] ${step}${message ? ": " + message : ""}${pct != null ? ` (${pct}%)` : ""}`;
        // Log to UI but do not forward to backend (already logged there)
        debugLog("debug", msg, true);
        const text = $("overlay-text");
        if (text && message) text.textContent = message;
        updateProgress(action, pct ?? 0, step, message || "");
      })
      .catch(() => {});
  }
}

// --- Debug console and logging ---
function debugLog(level, message, noForward = false) {
  const order = { error: 0, warn: 1, info: 2, debug: 3, trace: 4 };
  const allowed = state.logLevel === "trace" ? 4 : (state.logLevel === "debug" ? 3 : 2);
  const lvl = order[String(level).toLowerCase()] ?? 2;
  // Show in UI if allowed
  if (lvl <= allowed) {
    const el = $("debug-log");
    if (el) {
      const ts = new Date().toLocaleTimeString();
      const line = `[${ts}] ${String(level).toUpperCase()}: ${message}\n`;
      el.textContent += line;
      el.scrollTop = el.scrollHeight;
    }
  }
  // Always forward to backend (unless suppressed) to keep file logs maximal
  if (!noForward) {
    try {
      tauriInvoke("log_message", { level: String(level), message: String(message) }).catch(() => {});
    } catch {}
  }
}

function initDebugUI() {
  $("debug-clear").addEventListener("click", () => {
    $("debug-log").textContent = "";
  });

  // Modal controls
  // modal version removed (migrated to inline panel)
  const copyModal = $("copy-log-path"); if (copyModal) copyModal.addEventListener("click", async () => { const p = $("log-path").textContent; try { await navigator.clipboard.writeText(p); showToast("Copied"); } catch {} });
  // Inline toolbar bindings (panel)
  const setLogPath = (p) => {
    const el1 = $("log-path"); if (el1) el1.textContent = p;
    const el2 = $("log-path-inline"); if (el2) el2.textContent = p;
  };
  try {
    const info = await tauriInvoke("get_log_info");
    if (info?.logPath || info?.log_path) setLogPath(info.logPath || info.log_path);
  } catch {}
  const copyInline = $("copy-log-path-inline"); if (copyInline) copyInline.addEventListener("click", async () => { const p = $("log-path-inline").textContent; try { await navigator.clipboard.writeText(p); showToast("Copied"); } catch {} });
  const openInline = $("open-log-folder-inline"); if (openInline) openInline.addEventListener("click", () => { const p = $("log-path-inline").textContent; if (p) openPath(p.replace(/\\/g, "/").replace(/\/[^/]*$/, "/")); });
  const saveInline = $("save-console-inline"); if (saveInline) saveInline.addEventListener("click", async () => {
    try {
      const info = await tauriInvoke("get_log_info");
      const def = (info?.logPath || info?.log_path || "gui.log").replace(/gui\.log$/, "console-buffer.txt");
      const path = await tauriDialog().save({ defaultPath: def });
      if (!path) return;
      const content = $("debug-log").textContent || "";
      await tauriInvoke("save_text_file", { path, content });
      showToast(`Saved: ${path}`);
    } catch (e) { showError(e); }
  });
  const collInline = $("collect-diagnostics-inline"); if (collInline) collInline.addEventListener("click", async () => {
    try {
      const info = await tauriInvoke("get_log_info");
      const def = (info?.logPath || info?.log_path || "gui.log").replace(/gui\.log$/, "diagnostics.txt");
      const path = await tauriDialog().save({ defaultPath: def });
      if (!path) return;
      const saved = await tauriInvoke("collect_diagnostics", { req: { out_path: path } });
      showToast(`Diagnostics saved: ${saved}`);
    } catch (e) { showError(e); }
  });
  const simErrInline = $("simulate-error-inline"); if (simErrInline) simErrInline.addEventListener("click", async () => { try { await tauriInvoke("simulate_error"); } catch (e) { showError(e); } });
  const simPanInline = $("simulate-panic-inline"); if (simPanInline) simPanInline.addEventListener("click", async () => { try { await tauriInvoke("simulate_panic"); } catch (e) { showError(e); } });
  const lvInline = $("log-level-inline"); if (lvInline) lvInline.addEventListener("change", async () => { state.logLevel = lvInline.value; localStorage.setItem("rimloc.logLevel", state.logLevel); try { await tauriInvoke("set_debug_options", { opts: { minLevel: state.logLevel } }); } catch {} });
  const btInline = $("enable-backtrace-inline"); if (btInline) { btInline.checked = localStorage.getItem("rimloc.backtrace") === "1"; btInline.addEventListener("change", async () => { const on = btInline.checked; localStorage.setItem("rimloc.backtrace", on?"1":"0"); try { await tauriInvoke("set_debug_options", { opts: { backtrace: on } }); } catch {} }); }
  $("open-log-folder").addEventListener("click", () => {
    const path = $("log-path").textContent;
    if (path) openPath(path.replace(/\\/g, "/").replace(/\/[^/]*$/, "/"));
  });
  const saveModal = $("save-console"); if (saveModal) saveModal.addEventListener("click", async () => {
    try {
      const info = await tauriInvoke("get_log_info");
      const def = (info?.logPath || info?.log_path || "gui.log").replace(/gui\.log$/, "console-buffer.txt");
      const path = await tauriDialog().save({ defaultPath: def });
      if (!path) return;
      const content = $("debug-log").textContent || "";
      await tauriInvoke("save_text_file", { path, content });
      showToast(`Saved: ${path}`);
    } catch (e) {
      showError(e);
    }
  });
  const collectModal = $("collect-diagnostics"); if (collectModal) collectModal.addEventListener("click", async () => {
    try {
      const info = await tauriInvoke("get_log_info");
      const def = (info?.logPath || info?.log_path || "gui.log").replace(/gui\.log$/, "diagnostics.txt");
      const path = await tauriDialog().save({ defaultPath: def });
      if (!path) return;
      const saved = await tauriInvoke("collect_diagnostics", { req: { outPath: path } });
      showToast(`Diagnostics saved: ${saved}`);
    } catch (e) { showError(e); }
  });
  const simErrModal = $("simulate-error"); if (simErrModal) simErrModal.addEventListener("click", async () => {
    try { await tauriInvoke("simulate_error"); } catch (e) { showError(e); }
  });
  const simPanModal = $("simulate-panic"); if (simPanModal) simPanModal.addEventListener("click", async () => {
    try { await tauriInvoke("simulate_panic"); } catch (e) { showError(e); }
  });
  const lvModal = $("log-level-modal"); if (lvModal) lvModal.addEventListener("change", async () => { const lv = lvModal.value; state.logLevel = lv; localStorage.setItem("rimloc.logLevel", lv); try { await tauriInvoke("set_debug_options", { opts: { minLevel: lv } }); } catch {} });
  const btModal = $("enable-backtrace"); if (btModal) btModal.addEventListener("change", async () => { const on = btModal.checked; localStorage.setItem("rimloc.backtrace", on?"1":"0"); try { await tauriInvoke("set_debug_options", { opts: { backtrace: on } }); } catch {} });

  // Hotkey: Cmd/Ctrl + D opens modal
  // Hotkey disabled (modal removed); could focus debug log or toggle panel if needed
}

// --- Progress panel ---
function ensureProgressRow(action) {
  const list = $("progress-items");
  let row = list.querySelector(`[data-action="${action}"]`);
  if (row) return row;
  row = document.createElement("div");
  row.className = "progress-row";
  row.dataset.action = action;
  const label = document.createElement("div");
  label.className = "progress-label";
  label.textContent = tr(action) || action;
  const bar = document.createElement("div");
  bar.className = "progress-bar";
  const fill = document.createElement("div");
  fill.className = "progress-fill";
  bar.appendChild(fill);
  const pct = document.createElement("div");
  pct.className = "progress-pct";
  row.append(label, bar, pct);
  list.appendChild(row);
  return row;
}

function updateProgress(action, pct, step, message) {
  state.progress[action] = { pct, step, message };
  const row = ensureProgressRow(action);
  const fill = row.querySelector(".progress-fill");
  const pctEl = row.querySelector(".progress-pct");
  fill.style.width = `${Math.max(0, Math.min(100, pct))}%`;
  pctEl.textContent = `${pct}%`;
}

// --- i18n ---
const I18N = {
  en: {
    settings: "Settings",
    title: "RimLoc GUI",
    subtitle: "Desktop companion for RimLoc services",
    language: "Language",
    theme: "Theme",
    loglevel: "Log level",
    mod_setup: "Mod setup",
    mod_setup_hint: "Choose the mod folder and optional game version. Values are stored locally for convenience.",
    mod_root: "Mod root",
    browse: "Browse…",
    game_version: "Game version (optional)",
    target_lang: "Target language code",
    source_lang: "Source language code",
    scan_title: "Scan Keyed & DefInjected",
    scan_run: "Run scan",
    scan_save_json: "Save JSON…",
    scan_save_csv: "Save CSV…",
    scan_all_versions: "Include all versions under root",
    scan_defs_root: "Defs root (optional)",
    scan_extra_fields: "Extra fields (comma-separated)",
    scan_defs_dicts: "Defs dictionaries (one per line)",
    scan_type_schema: "Type schema (optional)",
    scan_keyed_nested: "Nested Keyed (dot-paths)",
    scan_no_inherit: "No inheritance",
    scan_plugins: "Run plugins",
    scan_empty: "No scan performed yet.",
    th_key: "Key",
    th_kind: "Kind",
    th_source: "Source sample",
    th_path: "Path",
    th_line: "Line",
    learn_title: "Learn DefInjected",
    learn_run: "Learn defs",
    open_output: "Open output",
    out_folder: "Output folder",
    source_lang_folder: "Source language folder",
    learn_empty: "No learn run yet.",
    export_title: "Export PO",
    export_run: "Export PO",
    open_file: "Open file",
    po_out_file: "PO output file",
    tm_folders: "Translation memory folders (one per line)",
    export_pot: "Write POT (template)",
    export_empty: "No export performed yet.",
    save_report: "Save report…",
    toast_learned: "Learned DefInjected candidates",
    build_title: "Build Translation Mod",
    build_run: "Build",
    build_dedupe: "Dedupe keys",
    build_empty: "No build performed yet.",
    from_root: "From existing root (optional)",
    from_versions: "Only game versions (comma-separated)",
    validate_title: "Validate",
    validate_run: "Run validate",
    validate_empty: "No validation run yet.",
    validate_compare_ph: "Compare placeholders EN↔Target",
    defs_root: "Defs root (optional)",
    extra_fields: "Extra fields (comma-separated)",
    health_title: "XML Health",
    health_run: "Check",
    health_empty: "No health check yet.",
    health_strict: "Strict mode (non-empty=error)",
    health_only: "Only categories",
    health_except: "Except categories",
    preview_title: "Preview EN → Target",
    preview_missing_only: "Missing only",
    import_title: "Import PO → XML",
    import_run: "Import PO",
    po_file: "PO file",
    target_lang_folder: "Target language folder",
    import_single_file: "Single file (_Imported.xml)",
    import_incremental: "Incremental (skip identical)",
    import_keep_empty: "Keep empty entries",
    import_only_diff: "Only changed keys",
    import_report: "Print summary report",
    import_empty: "No import performed yet.",
    diff_title: "Diff XML",
    diff_run: "Run diff",
    diff_empty: "No diff run yet.",
    lang_update_title: "Lang Update",
    lang_update_run: "Update",
    lang_update_empty: "No lang update yet.",
    lu_game_root: "Game root (folder with Data)",
    dry_run: "Dry run",
    backup: "Backup existing",
    annotate_title: "Annotate Keyed",
    annotate_preview: "Preview",
    annotate_apply: "Apply",
    annotate_empty: "No annotate yet.",
    strip_comments: "Strip existing comments",
    init_title: "Init Language",
    init_run: "Init",
    init_empty: "No init yet.",
    overwrite: "Overwrite existing",
    debug_console: "Debug console",
    clear: "Clear",
    open_debug: "Debug…",
    debug_modal_title: "Diagnostics",
    debug_modal_hint: "Copy errors or share with maintainer. Log file:",
    copy: "Copy",
    open_folder: "Open folder",
    save_console: "Save console…",
    progress: "Progress",
    scan: "Scan",
    learn: "Learn",
    export: "Export",
    type_schema: "Type schema (optional)",
    morph_provider: "Provider",
    morph_token: "Morpher token",
    morph_filter: "Filter key regex",
    timeout_ms: "Timeout, ms",
    cache_size: "Cache size",
    pymorphy_url: "Pymorphy URL",
    dict_files: "Dict files (one per line)",
    blacklist: "Blacklist (one per line)",
    exclude_substrings: "Exclude substrings (one per line)",
    min_length: "Min length",
    must_letter: "Must contain letter",
    from_defs: "From Defs (special)",
    schemas_title: "JSON Schemas",
    schemas_dump: "Dump schemas…",
    game_root: "Game root (folder with Data)",
    repo: "Repo",
    branch: "Branch",
    run_plugins: "Run plugins",
    morph_title: "Morph (Cases/Plural/Gender)",
    morph_generate: "Generate",
    limit: "Limit",
    learn_keyed_title: "Learn Keyed",
    run: "Run",
    validate: "Validate",
    health: "XML Health",
    import: "Import",
    build: "Build",
    diff: "Diff",
    annotate: "Annotate",
    init: "Init",
    lang_update: "Lang Update",
    footer_hint: "Need DefInjected strings? Run “Learn defs” first and copy suggested.xml into your language folder.",
    "lang.auto": "Auto",
    "theme.auto": "Auto",
    "theme.light": "Light",
    "theme.dark": "Dark",
    // placeholders
    ph: "",
    "ph.path.mod": "/path/to/mod",
    "ph.eg_game_ver": "e.g. 1.5",
    "ph.lang_ru": "ru",
    "ph.lang_auto": "auto",
    "ph.filter_keys": "filter keys…",
    "ph.path.defs": "/path/to/Defs",
    "ph.extra_fields": "label,description",
    "ph.dicts_lines": "/path/to/dict.json\n/path/to/another.json",
    "ph.path.schema": "/path/to/schema.json",
    "ph.out_dir": "_learn",
    "ph.lang_en": "English",
    "ph.threshold08": "0.8",
    "ph.path.po": "_learn/translation.po",
    "ph.tm_roots": "/path/to/reference-mod",
    "ph.lang_ru_dir": "Russian",
    "ph.categories_only_csv": "duplicate,empty,placeholder-check",
    "ph.categories_except_csv": "placeholder-check",
    "ph.path.imported_xml": "/path/to/_Imported.xml",
    "ph.path.out_mod": "/path/to/mod-out",
    "ph.mod_name": "RimLoc Translation",
    "ph.package_id": "your.name.rimloc",
    "ph.rw_version": "1.5",
    "ph.path.baseline_po": "/path/to/baseline.po",
    "ph.regex_all": ".*",
    "ph.timeout_ms": "1500",
    "ph.cache_size": "1024",
    "ph.pym_url": "http://localhost:5000",
    "ph.blacklist_lines": "prefix_.*\n^Key$",
    "ph.exclude_lines": "TODO\nWIP",
    "ph.min_length_1": "1",
    "ph.path.game_root": "/path/to/RimWorld",
    "ph.schemas_out": "./docs/assets/schemas",
  },
  ru: {
    settings: "Настройки",
    title: "RimLoc GUI",
    subtitle: "Настольный интерфейс для сервисов RimLoc",
    language: "Язык",
    theme: "Тема",
    loglevel: "Уровень логов",
    mod_setup: "Настройка мода",
    mod_setup_hint: "Выберите папку мода и, при необходимости, версию игры. Значения сохраняются локально.",
    mod_root: "Корень мода",
    browse: "Выбрать…",
    game_version: "Версия игры (опционально)",
    target_lang: "Целевой язык",
    source_lang: "Язык-источник",
    scan_title: "Сканирование Keyed и DefInjected",
    scan_run: "Сканировать",
    scan_save_json: "Сохранить JSON…",
    scan_save_csv: "Сохранить CSV…",
    scan_all_versions: "Включить все версии в корне",
    scan_defs_root: "Папка Defs (опционально)",
    scan_extra_fields: "Доп. поля (через запятую)",
    scan_defs_dicts: "Словари Defs (по одному в строке)",
    scan_type_schema: "Схема типов (опционально)",
    scan_keyed_nested: "Nested Keyed (dot-paths)",
    scan_no_inherit: "Без наследования",
    scan_plugins: "Запуск плагинов",
    scan_empty: "Сканирование ещё не выполнялось.",
    th_key: "Ключ",
    th_kind: "Тип",
    th_source: "Источник",
    th_path: "Путь",
    th_line: "Строка",
    learn_title: "Обучение DefInjected",
    learn_run: "Обучить",
    open_output: "Открыть папку",
    out_folder: "Папка вывода",
    source_lang_folder: "Папка исходного языка",
    learn_empty: "Обучение ещё не выполнялось.",
    export_title: "Экспорт PO",
    export_run: "Экспортировать PO",
    open_file: "Открыть файл",
    po_out_file: "Файл PO",
    tm_folders: "Папки TM (по одной в строке)",
    export_pot: "Писать POT (шаблон)",
    export_empty: "Экспорт ещё не выполнялся.",
    save_report: "Сохранить отчёт…",
    toast_learned: "Выбраны кандидаты DefInjected",
    build_title: "Сборка перевода из PO",
    build_run: "Собрать",
    build_dedupe: "Удалять дубли ключей",
    build_empty: "Сборка ещё не выполнялась.",
    from_root: "Из существующего корня (опционально)",
    from_versions: "Только версии игры (через запятую)",
    validate_title: "Валидация",
    validate_run: "Проверить",
    validate_empty: "Валидация ещё не выполнялась.",
    validate_compare_ph: "Сравнивать плейсхолдеры EN↔Целевой",
    defs_root: "Папка Defs (опционально)",
    extra_fields: "Доп. поля (через запятую)",
    health_title: "Проверка XML",
    health_run: "Проверить",
    health_empty: "Проверка ещё не выполнялась.",
    health_strict: "Строгий режим (наличие проблем=ошибка)",
    health_only: "Только категории",
    health_except: "Исключить категории",
    preview_title: "Предпросмотр EN → Целевой",
    preview_missing_only: "Только отсутствующие",
    import_title: "Импорт PO → XML",
    import_run: "Импортировать PO",
    po_file: "Файл PO",
    target_lang_folder: "Папка целевого языка",
    import_single_file: "Один файл (_Imported.xml)",
    import_incremental: "Инкрементально (пропуск идентичных)",
    import_keep_empty: "Сохранять пустые",
    import_only_diff: "Только изменённые ключи",
    import_report: "Печатать отчёт",
    import_empty: "Импорт ещё не выполнялся.",
    diff_title: "Сравнение XML",
    diff_run: "Сравнить",
    diff_empty: "Сравнение ещё не выполнялось.",
    lang_update_title: "Обновление языка",
    lang_update_run: "Обновить",
    lang_update_empty: "Обновление ещё не выполнялось.",
    lu_game_root: "Папка игры (с папкой Data)",
    dry_run: "Черновой запуск",
    backup: "Делать бэкап",
    annotate_title: "Аннотация Keyed",
    annotate_preview: "Предпросмотр",
    annotate_apply: "Применить",
    annotate_empty: "Аннотация ещё не выполнялась.",
    strip_comments: "Удалять существующие комментарии",
    init_title: "Инициализация языка",
    init_run: "Инициализировать",
    init_empty: "Инициализация ещё не выполнялась.",
    overwrite: "Перезаписывать существующие",
    debug_console: "Консоль отладки",
    clear: "Очистить",
    open_debug: "Отладка…",
    debug_modal_title: "Диагностика",
    debug_modal_hint: "Скопируйте ошибки или поделитесь с мейнтейнером. Файл логов:",
    copy: "Скопировать",
    open_folder: "Открыть папку",
    save_console: "Сохранить консоль…",
    progress: "Прогресс",
    scan: "Сканирование",
    learn: "Обучение",
    export: "Экспорт",
    type_schema: "Схема типов (опционально)",
    morph_provider: "Провайдер",
    morph_token: "Токен Morpher",
    morph_filter: "Regex фильтр ключей",
    timeout_ms: "Таймаут, мс",
    cache_size: "Размер кэша",
    pymorphy_url: "URL Pymorphy",
    dict_files: "Файлы словарей (по одному в строке)",
    blacklist: "Стоп‑слова/ключи (по одному в строке)",
    exclude_substrings: "Исключать подстроки (по одной в строке)",
    min_length: "Мин. длина",
    must_letter: "Должна быть буква",
    from_defs: "Из Defs (спец.)",
    schemas_title: "JSON схемы",
    schemas_dump: "Выгрузить схемы…",
    game_root: "Папка игры (с Data)",
    repo: "Репозиторий",
    branch: "Ветка",
    run_plugins: "Запуск плагинов",
    morph_title: "Морфология (падеж/мн.число/род)",
    morph_generate: "Сгенерировать",
    limit: "Лимит",
    learn_keyed_title: "Обучение Keyed",
    run: "Запуск",
    validate: "Проверка",
    health: "Проверка XML",
    import: "Импорт",
    build: "Сборка",
    diff: "Сравнение",
    annotate: "Аннотация",
    init: "Инициализация",
    lang_update: "Обновление языка",
    footer_hint: "Нужны строки DefInjected? Сначала выполните “Обучение”, затем скопируйте suggested.xml в папку вашего языка.",
    "lang.auto": "Авто",
    "theme.auto": "Авто",
    "theme.light": "Светлая",
    "theme.dark": "Тёмная",
    // placeholders
    ph: "",
    "ph.path.mod": "/путь/к/моду",
    "ph.eg_game_ver": "например, 1.5",
    "ph.lang_ru": "ru",
    "ph.lang_auto": "auto",
    "ph.filter_keys": "поиск по ключам…",
    "ph.path.defs": "/путь/к/Defs",
    "ph.extra_fields": "label,description",
    "ph.dicts_lines": "/путь/к/dict.json\n/путь/к/another.json",
    "ph.path.schema": "/путь/к/schema.json",
    "ph.out_dir": "_learn",
    "ph.lang_en": "English",
    "ph.threshold08": "0.8",
    "ph.path.po": "_learn/translation.po",
    "ph.tm_roots": "/путь/к/референс-моду",
    "ph.lang_ru_dir": "Russian",
    "ph.categories_only_csv": "duplicate,empty,placeholder-check",
    "ph.categories_except_csv": "placeholder-check",
    "ph.path.imported_xml": "/путь/к/_Imported.xml",
    "ph.path.out_mod": "/путь/к/mod-out",
    "ph.mod_name": "RimLoc Translation",
    "ph.package_id": "your.name.rimloc",
    "ph.rw_version": "1.5",
    "ph.path.baseline_po": "/путь/к/baseline.po",
    "ph.regex_all": ".*",
    "ph.timeout_ms": "1500",
    "ph.cache_size": "1024",
    "ph.pym_url": "http://localhost:5000",
    "ph.blacklist_lines": "prefix_.*\n^Key$",
    "ph.exclude_lines": "TODO\nWIP",
    "ph.min_length_1": "1",
    "ph.path.game_root": "/путь/к/RimWorld",
    "ph.schemas_out": "./docs/assets/schemas",
  },
};

function detectLocale() {
  if (state.locale !== "auto") return state.locale;
  const nav = navigator.language || "en";
  if (nav.toLowerCase().startsWith("ru")) return "ru";
  return "en";
}

function tr(key) {
  const loc = detectLocale();
  return I18N[loc]?.[key] ?? I18N.en[key] ?? key;
}

function applyI18n() {
  document.querySelectorAll("[data-i18n]").forEach((el) => {
    const key = el.getAttribute("data-i18n");
    const text = tr(key);
    if (!text) return;
    // Если это label с полями ввода — не перетираем разметку, а добавляем текст в отдельный span
    if (el.tagName === 'LABEL' && el.querySelector('input,textarea,select')) {
      let span = el.querySelector('.i18n-label');
      if (!span) {
        span = document.createElement('span');
        span.className = 'i18n-label';
        el.insertBefore(span, el.firstChild);
      }
      span.textContent = text;
    } else {
      el.textContent = text;
    }
  });
  // options in selects are handled via data-i18n on options
  // Localize placeholders via data-ph="i18n.key"
  document.querySelectorAll('[data-ph]').forEach((el) => {
    const key = el.getAttribute('data-ph');
    const text = tr(key);
    if (text) el.setAttribute('placeholder', text);
  });
}

function initI18nUI() {
  const select = $("locale-select");
  select.value = state.locale;
  select.addEventListener("change", () => {
    state.locale = select.value;
    localStorage.setItem("rimloc.locale", state.locale);
    applyI18n();
  });
  applyI18n();
}

// --- Theme ---
function applyTheme() {
  const html = document.documentElement;
  let mode = state.theme;
  if (mode === "auto") {
    mode = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches ? "dark" : "light";
  }
  html.dataset.theme = mode;
}

function initThemeUI() {
  const select = $("theme-select");
  select.value = state.theme;
  select.addEventListener("change", () => {
    state.theme = select.value;
    localStorage.setItem("rimloc.theme", state.theme);
    applyTheme();
  });
  if (window.matchMedia) {
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
      if (state.theme === 'auto') applyTheme();
    });
  }
  applyTheme();
}

function boot() {
  try { console.log('Boot: begin'); } catch {}
  try {
    if (!getTauri().core && !getTauri().invoke && !(getTauri().tauri && getTauri().tauri.invoke)) {
      try { console.warn('TAURI API not injected (withGlobalTauri?)'); } catch {}
      showToast('Tauri API not available (check withGlobalTauri)', true);
    }
    initPersistence();
    initEventHandlers();
    initDebugUI();
    initI18nUI();
    initThemeUI();
    renderScan(null);
    if (typeof renderPreview === 'function') renderPreview(null);
    renderLearn(null);
    renderExport(null);
    fetchAppVersion();
    debugLog("info", "UI ready", true);
    try { console.log('Boot: complete'); } catch {}
  } catch (e) {
    try { console.error('Boot error', e); } catch {}
    showError(e);
  }
  // capture unhandled errors
  window.addEventListener("error", (e) => {
    const msg = e?.error?.message || e?.message || String(e?.error || e);
    debugLog("error", `Unhandled error: ${msg}`);
  });
  window.addEventListener("unhandledrejection", (e) => {
    const r = e?.reason;
    const msg = r?.message || (typeof r === "string" ? r : (r ? JSON.stringify(r) : "unknown"));
    debugLog("error", `Unhandled rejection: ${msg}`);
  });
}

async function waitForTauri(maxMs = 5000) {
  const start = Date.now();
  while (Date.now() - start < maxMs) {
    const t = getTauri();
    if (t && (t.invoke || t.core?.invoke || t.tauri?.invoke)) return true;
    await new Promise(r => setTimeout(r, 50));
  }
  return false;
}

(async () => {
  if (document.readyState === 'loading') {
    await new Promise(r => document.addEventListener('DOMContentLoaded', r, { once: true }));
  }
  const ok = await waitForTauri();
  if (!ok) {
    try { console.warn('TAURI invoke not ready after timeout'); } catch {}
  }
  boot();
})();

// Автозаполнение путей по умолчанию при выборе корня
document.addEventListener('DOMContentLoaded', () => {
  const rootEl = $("mod-root");
  if (!rootEl) return;
  const ensureDefaults = () => {
    const root = val('mod-root');
    if (!root) return;
    if (!val('po-output')) $("po-output").value = `${root.replace(/\\/g,'/')}/_learn/translation.po`;
    if (isChecked('import-single-file') && !val('import-out-xml')) $("import-out-xml").value = `${root.replace(/\\/g,'/')}/_learn/_Imported.xml`;
  };
  rootEl.addEventListener('change', ensureDefaults);
  ensureDefaults();
});
  // Validate save
  const valSave = $("validate-save"); if (valSave) valSave.addEventListener("click", async () => {
    const root = val("mod-root"); if (!root) return showToast("Select mod root first", true);
    const path = await tauriDialog().save({ defaultPath: `${root.replace(/\\/g,'/')}/_learn/validate.json` });
    if (!path) return; const payload = {
      root,
      game_version: val("game-version") || null,
      source_lang: val("validate-source-lang") || null,
      source_lang_dir: val("validate-source-lang-dir") || null,
      defs_root: val("validate-defs-root") || null,
      extra_fields: (val("validate-extra-fields") || "").split(',').map(s => s.trim()).filter(Boolean),
      out_json: path,
    };
    await runAction("Saving validation…", () => tauriInvoke("validate_mod", { request: payload }));
    showToast(`Saved: ${path}`);
  });

  // Diff save
  const diffSave = $("diff-save"); if (diffSave) diffSave.addEventListener("click", async () => {
    const root = val("mod-root"); if (!root) return showToast("Select mod root first", true);
    const path = await tauriDialog().save({ defaultPath: `${root.replace(/\\/g,'/')}/_learn/diff.json` });
    if (!path) return; const payload = {
      root,
      game_version: val("game-version") || null,
      source_lang_dir: val("diff-source-lang-dir") || "English",
      target_lang_dir: val("diff-target-lang-dir") || "Russian",
      defs_root: val("diff-defs-root") || null,
      baseline_po: val("diff-po") || null,
      out_json: path,
    };
    await runAction("Saving diff…", () => tauriInvoke("diff_xml_cmd", { request: payload }));
    showToast(`Saved: ${path}`);
});

// === Preview panel ===
function collectByKey(units, sourceLangDir, targetLangDir) {
  const map = new Map();
  const norm = (s) => (s || '').trim();
  for (const u of units || []) {
    const path = String(u.path || '');
    const key = u.key;
    if (!key) continue;
    let rec = map.get(key);
    if (!rec) { rec = { key, en: '', trg: '', enPath: '', trgPath: '' }; map.set(key, rec); }
    if (path.includes(`/Languages/${sourceLangDir}/`) || path.includes(`\\Languages\\${sourceLangDir}\\`) || path.includes('/Defs/') || path.includes('\\Defs\\')) {
      rec.en = norm(u.source || u.value || ''); rec.enPath = u.path;
    }
    if (path.includes(`/Languages/${targetLangDir}/`) || path.includes(`\\Languages\\${targetLangDir}\\`)) {
      rec.trg = norm(u.source || u.value || ''); rec.trgPath = u.path;
    }
  }
  return Array.from(map.values());
}

function renderPreview(rows) {
  const list = $("preview-keys"); if (!list) return;
  const root = val("mod-root");
  const trg = val("target-lang") || 'ru';
  const trgDir = langToDir(trg) || 'Russian';
  const srcDir = 'English';
  const units = window._lastScanUnits || [];
  const data = collectByKey(units, srcDir, trgDir);
  const term = (val("preview-filter") || '').toLowerCase();
  const missingOnly = isChecked("preview-missing-only");
  list.innerHTML = '';
  let shown = 0;
  for (const r of data) {
    if (term && !r.key.toLowerCase().includes(term)) continue;
    if (missingOnly && r.en && r.trg) continue;
    const tr = document.createElement('tr');
    tr.innerHTML = `<td>${escapeHtml(r.key)}</td><td>${escapeHtml(r.en)}</td><td>${escapeHtml(r.trg)}</td>`;
    tr.addEventListener('click', () => {
      $("preview-en").textContent = r.en || '';
      $("preview-target").textContent = r.trg || '';
    });
    list.appendChild(tr); shown++;
    if (shown > 300) break;
  }
}

function escapeHtml(s) {
  return String(s || '').replace(/[&<>"']/g, (c) => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;','\'':'&#39;'}[c]));
}

function langToDir(code) {
  const l = (code || '').trim().toLowerCase();
  const map = { 'ru':'Russian','en':'English','ja':'Japanese','ko':'Korean','fr':'French','de':'German','es':'Spanish','pt-br':'PortugueseBrazilian','pt':'Portuguese','pl':'Polish','it':'Italian','tr':'Turkish','uk':'Ukrainian','cs':'Czech','hu':'Hungarian','nl':'Dutch','ro':'Romanian','th':'Thai','el':'Greek','zh':'ChineseSimplified','zh-tw':'ChineseTraditional' };
  return map[l];
}

const previewFilter = $("preview-filter"); if (previewFilter) previewFilter.addEventListener('input', () => renderPreview());
const missingToggle = $("preview-missing-only"); if (missingToggle) missingToggle.addEventListener('change', () => renderPreview());
