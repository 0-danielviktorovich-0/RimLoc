function getTauri() {
  return window.__TAURI__ || {};
}

function tauriInvoke(cmd, args) {
  const tauri = getTauri();
  const fn = tauri.invoke || tauri.tauri?.invoke;
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
      debugLog("warn", `dialog.open failed, fallback to backend: ${formatError(e)}`);
    }
    try {
      // Fallback to backend command
      const selected2 = await tauriInvoke("pick_directory", { initial: current });
      if (selected2) {
        $(targetId).value = selected2;
        $(targetId).dispatchEvent(new Event("change"));
        showToast(selected2);
      }
    } catch (e2) {
      showError(e2);
    }
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
      showError(e);
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
    gameVersion: $("game-version").value.trim() || null,
    lang: $("target-lang").value.trim() || null,
  };
  if (saveMode === "json") {
    const path = await tauriDialog().save({
      defaultPath: `${root.replace(/\\/g, "/")}/_learn/scan.json`,
    });
    if (!path) return;
    payload.outJson = path;
    await runAction("Saving JSON…", () => tauriInvoke("scan_mod", payload));
    showToast(`Saved scan JSON to ${path}`);
    return;
  }
  if (saveMode === "csv") {
    const path = await tauriDialog().save({
      defaultPath: `${root.replace(/\\/g, "/")}/_learn/scan.csv`,
    });
    if (!path) return;
    payload.outCsv = path;
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
    outDir: outDir || null,
    langDir: langDir || null,
    gameVersion: $("game-version").value.trim() || null,
  };
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
    outPo,
    lang: $("target-lang").value.trim() || null,
    sourceLang: $("source-lang").value.trim() || null,
    tmRoots: tmRoots.length ? tmRoots : null,
    gameVersion: $("game-version").value.trim() || null,
  };
  const result = await runAction("Exporting PO…", () => tauriInvoke("export_po", payload));
  renderExport(result);
  showToast("PO exported successfully");
  updateStatus("Export finished");
}

function openPath(path) {
  if (!path) return;
  tauriShell()
    .open(path)
    .catch((err) => showError(err));
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
        debugLog(payload.level || "info", payload.message || String(payload));
      })
      .catch(() => {});

    ev
      .listen("progress", ({ payload }) => {
        if (!payload) return;
        const { action, step, message, pct } = payload;
        const msg = `[${action}] ${step}${message ? ": " + message : ""}${pct != null ? ` (${pct}%)` : ""}`;
        debugLog("debug", msg);
        const text = $("overlay-text");
        if (text && message) text.textContent = message;
      })
      .catch(() => {});
  }
}

// --- Debug console and logging ---
function debugLog(level, message) {
  const order = { error: 0, warn: 1, info: 2, debug: 3 };
  const allowed = state.logLevel === "debug" ? 3 : 2;
  const lvl = order[String(level).toLowerCase()] ?? 2;
  if (lvl > allowed) return;
  const el = $("debug-log");
  if (!el) return;
  const ts = new Date().toLocaleTimeString();
  const line = `[${ts}] ${String(level).toUpperCase()}: ${message}\n`;
  el.textContent += line;
  el.scrollTop = el.scrollHeight;
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
    if (path) tauriShell().open(path.replace(/\\/g, "/").replace(/\/[^/]*$/, "/"));
  });
}

// --- i18n ---
const I18N = {
  en: {
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
    debug_console: "Debug console",
    clear: "Clear",
    footer_hint: "Need DefInjected strings? Run “Learn defs” first and copy suggested.xml into your language folder.",
    "lang.auto": "Auto",
    "theme.auto": "Auto",
    "theme.light": "Light",
    "theme.dark": "Dark",
  },
  ru: {
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
    debug_console: "Консоль отладки",
    clear: "Очистить",
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
});
