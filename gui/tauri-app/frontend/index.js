function getTauri() {
  return window.__TAURI__ || {};
}

function tauriInvoke(cmd, args) {
  const tauri = getTauri();
  const fn = tauri.invoke || tauri.tauri?.invoke || tauri.core?.invoke;
  if (!fn) throw new Error("Tauri API not available: invoke");
  return fn(cmd, args);
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
  const root = $("mod-root").value.trim();
  if (!root) {
    showToast("Select mod root first", true);
    return;
  }
  const payload = {
    root,
    game_version: $("game-version").value.trim() || null,
    lang: $("target-lang").value.trim() || null,
    include_all_versions: $("scan-all-versions")?.checked || false,
  };
  debugLog("debug", `scan payload: ${JSON.stringify(payload)}`);
  if (saveMode === "json") {
    const path = await tauriDialog().save({
      defaultPath: `${root.replace(/\\/g, "/")}/_learn/scan.json`,
    });
    if (!path) return;
    payload.out_json = path;
    await runAction("Saving JSON…", () => tauriInvoke("scan_mod", payload));
    showToast(`Saved scan JSON to ${path}`);
    return;
  }
  if (saveMode === "csv") {
    const path = await tauriDialog().save({
      defaultPath: `${root.replace(/\\/g, "/")}/_learn/scan.csv`,
    });
    if (!path) return;
    payload.out_csv = path;
    await runAction("Saving CSV…", () => tauriInvoke("scan_mod", payload));
    showToast(`Saved scan CSV to ${path}`);
    return;
  }
  const result = await runAction("Scanning…", () => tauriInvoke("scan_mod", payload));
  renderScan(result);
  showToast(`Found ${result.total} entries`);
}

async function handleLearn() {
  const root = $("mod-root").value.trim();
  if (!root) {
    showToast("Select mod root first", true);
    return;
  }
  const outDir = $("learn-out").value.trim();
  const langDir = $("learn-lang-dir").value.trim();
  const payload = {
    root,
    out_dir: outDir || null,
    lang_dir: langDir || null,
    game_version: $("game-version").value.trim() || null,
  };
  debugLog("debug", `learn payload: ${JSON.stringify(payload)}`);
  const result = await runAction("Learning DefInjected…", () => tauriInvoke("learn_defs", payload));
  renderLearn(result);
  showToast("Learned DefInjected candidates");
  updateStatus("Learn completed");
}

async function handleExport() {
  const root = $("mod-root").value.trim();
  if (!root) {
    showToast("Select mod root first", true);
    return;
  }
  const outPo = $("po-output").value.trim();
  if (!outPo) {
    showToast("Specify output PO path", true);
    return;
  }
  const tmRoots = parseTmRoots($("tm-roots").value);
  const payload = {
    root,
    out_po: outPo,
    lang: $("target-lang").value.trim() || null,
    source_lang: $("source-lang").value.trim() || null,
    source_lang_dir: $("export-source-lang-dir").value.trim() || null,
    tm_roots: tmRoots.length ? tmRoots : null,
    game_version: $("game-version").value.trim() || null,
  };
  debugLog("debug", `export payload: ${JSON.stringify(payload)}`);
  const result = await runAction("Exporting PO…", () => tauriInvoke("export_po", payload));
  renderExport(result);
  showToast("PO exported successfully");
  updateStatus("Export finished");
}

async function handleValidate() {
  const root = $("mod-root").value.trim();
  if (!root) return showToast("Select mod root first", true);
  const payload = {
    root,
    game_version: $("game-version").value.trim() || null,
    source_lang: $("validate-source-lang").value.trim() || null,
    source_lang_dir: $("validate-source-lang-dir").value.trim() || null,
    defs_root: $("validate-defs-root").value.trim() || null,
    extra_fields: ($("validate-extra-fields").value || "").split(',').map(s => s.trim()).filter(Boolean),
  };
  debugLog("debug", `validate payload: ${JSON.stringify(payload)}`);
  const result = await runAction("Validating…", () => tauriInvoke("validate_mod", payload));
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
  const lines = (result.messages || []).slice(0, 500).map(m => `${(m.kind||'').toUpperCase()} ${m.path}${m.line?':'+m.line:''}: ${m.key} – ${m.message}`);
  pre.textContent = lines.join("\n");
  box.appendChild(pre);
}

async function handleHealth() {
  const root = $("mod-root").value.trim();
  if (!root) return showToast("Select mod root first", true);
  const payload = {
    root,
    game_version: $("game-version").value.trim() || null,
    lang_dir: $("health-lang-dir").value.trim() || null,
  };
  const result = await runAction("XML Health…", () => tauriInvoke("xml_health", payload));
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
  pre.textContent = (result.issues || []).slice(0, 500).map(i => `${i.path}${i.line?':'+i.line:''}: ${i.kind} – ${i.message}`).join("\n");
  box.appendChild(pre);
}

async function handleImport() {
  const root = $("mod-root").value.trim();
  if (!root) return showToast("Select mod root first", true);
  const po_path = $("import-po").value.trim();
  if (!po_path) return showToast("Select PO file", true);
  const payload = {
    root,
    po_path,
    game_version: $("game-version").value.trim() || null,
    lang_dir: $("import-lang-dir").value.trim() || null,
    keep_empty: $("import-keep-empty").checked,
    backup: false,
    single_file: $("import-single-file").checked,
    incremental: $("import-incremental").checked,
    only_diff: false,
    report: true,
  };
  debugLog("debug", `import payload: ${JSON.stringify(payload)}`);
  const result = await runAction("Importing PO…", () => tauriInvoke("import_po", payload));
  const box = $("import-result");
  box.textContent = `Created: ${result.created}, Updated: ${result.updated}, Skipped: ${result.skipped}, Keys: ${result.keys}`;
}

async function handleBuild() {
  const po_path = $("build-po").value.trim();
  const out_mod = $("build-out").value.trim();
  const lang_dir = $("build-lang-dir").value.trim() || "Russian";
  const name = $("build-name").value.trim() || "RimLoc Translation";
  const package_id = $("build-package").value.trim() || "your.name.rimloc";
  const rw_version = $("build-rw").value.trim() || "1.5";
  const dedupe = $("build-dedupe").checked;
  if (!po_path || !out_mod) return showToast("Select PO and output folder", true);
  const payload = { po_path, out_mod, lang_dir, name, package_id, rw_version, dedupe };
  debugLog("debug", `build payload: ${JSON.stringify(payload)}`);
  const result = await runAction("Building mod…", () => tauriInvoke("build_mod", payload));
  const box = $("build-result");
  if (box) box.textContent = `Built to ${result.outMod || result.out_mod}. Files: ${result.files}, Keys: ${result.totalKeys || result.total_keys || 0}`;
  showToast("Build complete");
}

async function handleDiff() {
  const root = $("mod-root").value.trim();
  if (!root) return showToast("Select mod root first", true);
  const payload = {
    root,
    game_version: $("game-version").value.trim() || null,
    source_lang_dir: $("diff-source-lang-dir").value.trim() || "English",
    target_lang_dir: $("diff-target-lang-dir").value.trim() || "Russian",
    defs_root: $("diff-defs-root").value.trim() || null,
    baseline_po: $("diff-po").value.trim() || null,
  };
  const res = await runAction("Diff XML…", () => tauriInvoke("diff_xml_cmd", payload));
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

async function handleLangUpdate() {
  const root = $("mod-root").value.trim();
  if (!root) return showToast("Select mod root first", true);
  const payload = {
    root,
    repo: $("lang-update-repo").value.trim() || "Ludeon/RimWorld",
    branch: $("lang-update-branch").value.trim() || null,
    source_lang_dir: $("lang-update-source").value.trim() || "English",
    target_lang_dir: $("lang-update-target").value.trim() || "Russian",
    dry_run: $("lang-update-dry").checked,
    backup: $("lang-update-backup").checked,
  };
  const res = await runAction("Lang update…", () => tauriInvoke("lang_update_cmd", payload));
  $("lang-update-result").textContent = `Files: ${res.files}, Bytes: ${res.bytes}, Out: ${res.outDir || res.out_dir}`;
}

async function handleAnnotate(dry) {
  const root = $("mod-root").value.trim();
  if (!root) return showToast("Select mod root first", true);
  const payload = {
    root,
    source_lang_dir: $("annotate-source").value.trim() || "English",
    target_lang_dir: $("annotate-target").value.trim() || "Russian",
    comment_prefix: $("annotate-prefix").value.trim() || "//",
    strip: $("annotate-strip").checked,
    dry_run: !!dry,
    backup: $("annotate-backup").checked,
  };
  const res = await runAction(dry ? "Annotate preview…" : "Annotate apply…", () => tauriInvoke("annotate_cmd", payload));
  $("annotate-result").textContent = `Processed: ${res.processed}, Annotated: ${res.annotated}`;
}

async function handleInit() {
  const root = $("mod-root").value.trim();
  if (!root) return showToast("Select mod root first", true);
  const payload = {
    root,
    source_lang_dir: $("init-source").value.trim() || "English",
    target_lang_dir: $("init-target").value.trim() || "Russian",
    overwrite: $("init-overwrite").checked,
    dry_run: $("init-dry").checked,
  };
  const res = await runAction("Init language…", () => tauriInvoke("init_lang_cmd", payload));
  $("init-result").textContent = `Files: ${res.files}, Language: ${res.outLanguage || res.out_language}`;
}

async function openPath(path) {
  if (!path) return;
  try {
    await tauriShell().open(path);
    return;
  } catch (err) {
    debugLog("warn", `shell.open not available: ${formatError(err)}`);
  }
  try {
    await tauriInvoke("open_path", { path });
  } catch (e) {
    showError(e);
  }
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
  document.querySelector('[data-action="pick-learn-out"]').addEventListener("click", pickDirectory("learn-out"));
  document.querySelector('[data-action="pick-po-output"]').addEventListener(
    "click",
    pickSave("po-output", { defaultPath: "translation.po" })
  );

  // Validate
  $("validate-run").addEventListener("click", handleValidate);
  const pickDefs = document.querySelector('[data-action="pick-validate-defs"]');
  if (pickDefs) pickDefs.addEventListener("click", pickDirectory("validate-defs-root"));

  // Health
  $("health-run").addEventListener("click", handleHealth);

  // Import
  $("import-run").addEventListener("click", handleImport);
  const pickImportPo = document.querySelector('[data-action="pick-import-po"]');
  if (pickImportPo) pickImportPo.addEventListener("click", () => pickFile("import-po", [{ name: "PO", extensions: ["po"] }])());

  // Build
  const buildRun = $("build-run");
  if (buildRun) buildRun.addEventListener("click", handleBuild);
  const pickBuildPo = document.querySelector('[data-action="pick-build-po"]');
  if (pickBuildPo) pickBuildPo.addEventListener("click", () => pickFile("build-po", [{ name: "PO", extensions: ["po"] }])());
  const pickBuildOut = document.querySelector('[data-action="pick-build-out"]');
  if (pickBuildOut) pickBuildOut.addEventListener("click", pickDirectory("build-out"));

  // Diff
  const diffRun = $("diff-run");
  if (diffRun) diffRun.addEventListener("click", handleDiff);
  const pickDiffDefs = document.querySelector('[data-action="pick-diff-defs"]');
  if (pickDiffDefs) pickDiffDefs.addEventListener("click", pickDirectory("diff-defs-root"));
  const pickDiffPo = document.querySelector('[data-action="pick-diff-po"]');
  if (pickDiffPo) pickDiffPo.addEventListener("click", () => pickFile("diff-po", [{ name: "PO", extensions: ["po"] }])());

  // Lang update
  const luRun = $("lang-update-run");
  if (luRun) luRun.addEventListener("click", handleLangUpdate);

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
  // Export options
  bindPersist("export-source-lang-dir", "rimloc.exportSourceLangDir", "English");
  // Validate options
  bindPersist("validate-source-lang", "rimloc.validateSourceLang");
  bindPersist("validate-source-lang-dir", "rimloc.validateSourceLangDir", "English");
  bindPersist("validate-defs-root", "rimloc.validateDefsRoot");
  bindPersist("validate-extra-fields", "rimloc.validateExtraFields");
  // Health options
  bindPersist("health-lang-dir", "rimloc.healthLangDir", "English");
  // Import options
  bindPersist("import-po", "rimloc.importPo");
  bindPersist("import-lang-dir", "rimloc.importLangDir", "Russian");
  const flags = ["import-single-file","import-incremental","import-keep-empty"];
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
  const buildDedupe = $("build-dedupe");
  if (buildDedupe) {
    buildDedupe.checked = localStorage.getItem("rimloc.buildDedupe") === "1";
    buildDedupe.addEventListener("change", () => localStorage.setItem("rimloc.buildDedupe", buildDedupe.checked ? "1" : "0"));
  }

  // Diff options
  bindPersist("diff-source-lang-dir", "rimloc.diffSource", "English");
  bindPersist("diff-target-lang-dir", "rimloc.diffTarget", "Russian");
  bindPersist("diff-defs-root", "rimloc.diffDefs");
  bindPersist("diff-po", "rimloc.diffPo");

  // Lang update options
  bindPersist("lang-update-repo", "rimloc.luRepo", "Ludeon/RimWorld");
  bindPersist("lang-update-branch", "rimloc.luBranch", "master");
  bindPersist("lang-update-source", "rimloc.luSource", "English");
  bindPersist("lang-update-target", "rimloc.luTarget", "Russian");
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
  const order = { error: 0, warn: 1, info: 2, debug: 3 };
  const allowed = state.logLevel === "debug" ? 3 : 2;
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
  $("log-level").value = state.logLevel;
  $("log-level").addEventListener("change", () => {
    state.logLevel = $("log-level").value;
    localStorage.setItem("rimloc.logLevel", state.logLevel);
  });
  $("debug-clear").addEventListener("click", () => {
    $("debug-log").textContent = "";
  });

  // Modal controls
  const modal = $("debug-modal");
  $("open-debug-modal").addEventListener("click", async () => {
    try {
      const info = await tauriInvoke("get_log_info");
      if (info?.logPath || info?.log_path) {
        $("log-path").textContent = info.logPath || info.log_path;
      }
    } catch {}
    $("debug-modal-content").textContent = $("debug-log").textContent;
    modal.classList.remove("hidden");
  });
  $("debug-modal-close").addEventListener("click", () => modal.classList.add("hidden"));
  modal.addEventListener("click", (e) => {
    if (e.target === modal) modal.classList.add("hidden");
  });
  $("copy-log-path").addEventListener("click", async () => {
    const path = $("log-path").textContent;
    try { await navigator.clipboard.writeText(path); showToast("Copied"); } catch {}
  });
  $("open-log-folder").addEventListener("click", () => {
    const path = $("log-path").textContent;
    if (path) openPath(path.replace(/\\/g, "/").replace(/\/[^/]*$/, "/"));
  });
  $("save-console").addEventListener("click", async () => {
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

  // Hotkey: Cmd/Ctrl + D opens modal
  document.addEventListener("keydown", (e) => {
    if ((e.metaKey || e.ctrlKey) && String(e.key).toLowerCase() === "d") {
      e.preventDefault();
      $("open-debug-modal").click();
    }
  });
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
    export_empty: "No export performed yet.",
    build_title: "Build Translation Mod",
    build_run: "Build",
    build_dedupe: "Dedupe keys",
    build_empty: "No build performed yet.",
    validate_title: "Validate",
    validate_run: "Run validate",
    validate_empty: "No validation run yet.",
    defs_root: "Defs root (optional)",
    extra_fields: "Extra fields (comma-separated)",
    health_title: "XML Health",
    health_run: "Check",
    health_empty: "No health check yet.",
    import_title: "Import PO → XML",
    import_run: "Import PO",
    po_file: "PO file",
    target_lang_folder: "Target language folder",
    import_single_file: "Single file (_Imported.xml)",
    import_incremental: "Incremental (skip identical)",
    import_keep_empty: "Keep empty entries",
    import_empty: "No import performed yet.",
    diff_title: "Diff XML",
    diff_run: "Run diff",
    diff_empty: "No diff run yet.",
    lang_update_title: "Lang Update",
    lang_update_run: "Update",
    lang_update_empty: "No lang update yet.",
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
    footer_hint: "Need DefInjected strings? Run “Learn defs” first and copy suggested.xml into your language folder.",
    "lang.auto": "Auto",
    "theme.auto": "Auto",
    "theme.light": "Light",
    "theme.dark": "Dark",
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
    export_empty: "Экспорт ещё не выполнялся.",
    build_title: "Сборка перевода из PO",
    build_run: "Собрать",
    build_dedupe: "Удалять дубли ключей",
    build_empty: "Сборка ещё не выполнялась.",
    validate_title: "Валидация",
    validate_run: "Проверить",
    validate_empty: "Валидация ещё не выполнялась.",
    defs_root: "Папка Defs (опционально)",
    extra_fields: "Доп. поля (через запятую)",
    health_title: "Проверка XML",
    health_run: "Проверить",
    health_empty: "Проверка ещё не выполнялась.",
    import_title: "Импорт PO → XML",
    import_run: "Импортировать PO",
    po_file: "Файл PO",
    target_lang_folder: "Папка целевого языка",
    import_single_file: "Один файл (_Imported.xml)",
    import_incremental: "Инкрементально (пропуск идентичных)",
    import_keep_empty: "Сохранять пустые",
    import_empty: "Импорт ещё не выполнялся.",
    diff_title: "Сравнение XML",
    diff_run: "Сравнить",
    diff_empty: "Сравнение ещё не выполнялось.",
    lang_update_title: "Обновление языка",
    lang_update_run: "Обновить",
    lang_update_empty: "Обновление ещё не выполнялось.",
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
    footer_hint: "Нужны строки DefInjected? Сначала выполните “Обучение”, затем скопируйте suggested.xml в папку вашего языка.",
    "lang.auto": "Авто",
    "theme.auto": "Авто",
    "theme.light": "Светлая",
    "theme.dark": "Тёмная",
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
    if (text) el.innerText = text;
  });
  // options in selects are handled via data-i18n on options
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

document.addEventListener("DOMContentLoaded", () => {
  initPersistence();
  initEventHandlers();
  initDebugUI();
  initI18nUI();
  initThemeUI();
  renderScan(null);
  renderLearn(null);
  renderExport(null);
  fetchAppVersion();
  debugLog("debug", "UI ready");
  // Capture and persist unhandled errors
  window.addEventListener("error", (e) => {
    const msg = e?.error?.message || e?.message || String(e?.error || e);
    debugLog("error", `Unhandled error: ${msg}`);
  });
  window.addEventListener("unhandledrejection", (e) => {
    const r = e?.reason;
    const msg = r?.message || (typeof r === "string" ? r : (r ? JSON.stringify(r) : "unknown"));
    debugLog("error", `Unhandled rejection: ${msg}`);
  });
});
