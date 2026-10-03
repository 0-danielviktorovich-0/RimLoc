// RimWorld Runtime Bridge v1.0.0 — TEST-ONLY, isolated clone profile ONLY.
// Part of AI-OS App Control Fabric: RimWorld Adapter → RimWorld Runtime Bridge.
// Never ship; capability compiled into the TEST mod artifact only.
//
// Protocol v1 (JSON envelope, file channel):
//   request  <savedata>/QA/commands/<id>.json
//     {"protocol_version":1,"request_id":"...","run_id":"...","op":"...","args":{...}}
//   response <savedata>/QA/results/<same-name>.json
//     {"protocol_version":1,"request_id":"...","run_id":"...","ok":true,
//      "result":{...},"error":null,"telemetry":{"frame":N,"state_revision":M,"wall_utc":"..."}}
//     ok=false → "error":{"code":"...","message":"...","detail":{...}}
//
// Ops (narrow allowlist, no shell, no arbitrary fs, no reflection):
//   capabilities                       → machine-readable manifest (Fabric capability schema entries)
//   state                              → runtime snapshot (version/language/mods/markers/revision)
//   select_language args.folder        → LanguageDatabase.SelectLanguage
//   research_tab                       → Find.MainTabsRoot.SetCurrentTab(Research) — colony only
//   read_def args.def                  → live ResearchProjectDef label/description
//   keyed args.key                     → live Translator lookup
//   screenshot args.path               → Unity ScreenCapture INSIDE evidence root + frame freshness
//   quit                               → Application.Quit
// Legacy v0.3 line-format commands/*.txt still accepted (deprecated, run_id unchecked).
//
// Error codes (semantic, never raw exception text as contract):
//   BAD_REQUEST, PROTOCOL_VERSION_UNSUPPORTED, RUN_ID_MISMATCH,
//   CAPABILITY_UNAVAILABLE, WRONG_GAME_STATE, NOT_IN_COLONY,
//   TARGET_DEF_NOT_FOUND, LANGUAGE_NOT_READY, EVIDENCE_ROOT_ESCAPE,
//   CAPTURE_TIMEOUT, INTERNAL.
// Full exception detail goes to the game log only, never into the API contract.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading;
using RimWorld;
using UnityEngine;
using Verse;

namespace RimLoc.RuntimeBridge
{
  [StaticConstructorOnStartup]
  public static class RuntimeBridge
  {
    private const string BridgeVersion = "1.0.0";
    private const int ProtocolVersion = 1;
    private const long MaxRequestBytes = 64 * 1024;
    private const int CaptureTimeoutMs = 30000;

    private static readonly string RunId = "run-" + DateTime.UtcNow.ToString("yyyyMMdd-HHmmss");
    private static string QaDir => Path.Combine(GenFilePaths.SaveDataFolderPath, "QA");
    private static string CommandsDir => Path.Combine(QaDir, "commands");
    private static string ResultsDir => Path.Combine(QaDir, "results");

    // Canonical Unity thread→main marshal: the player-loop synchronization
    // context executes posted actions every UPDATE, independent of RimWorld's
    // long-event queue (which at an occluded main menu may drain only after
    // minutes — live lesson of runs 065643/071216).
    private static readonly SynchronizationContext UnityCtx = SynchronizationContext.Current;

    // §5 frame freshness: bumped by every state-mutating semantic op so a
    // captured frame can be correlated with the state that requested it.
    private static int stateRevision;
    // §9 path fence: captures may land ONLY inside this root (default: QA dir).
    private static readonly string EvidenceRoot = ResolveEvidenceRoot();

    private static string ResolveEvidenceRoot()
    {
      try
      {
        string env = Environment.GetEnvironmentVariable("RIMLOC_BRIDGE_EVIDENCE_ROOT");
        if (!string.IsNullOrEmpty(env))
        {
          string full = Path.GetFullPath(env);
          if (Directory.Exists(full)) return full;
          Log.Warning("[RimLoc-RuntimeBridge] RIMLOC_BRIDGE_EVIDENCE_ROOT does not exist, falling back to QA dir: " + env);
        }
      }
      catch (Exception e) { Log.Warning("[RimLoc-RuntimeBridge] evidence root env unreadable: " + e.Message); }
      return null; // resolved lazily against QaDir (SaveDataFolderPath may not exist yet in ctor)
    }

    private static string Esc(string s)
    {
      if (s == null) return "null";
      var sb = new StringBuilder("\"");
      foreach (char c in s)
      {
        if (c == '"') sb.Append("\\\"");
        else if (c == '\\') sb.Append("\\\\");
        else if (c == '\n') sb.Append("\\n");
        else if (c == '\r') { }
        else if (c < ' ') sb.Append("\\u").Append(((int)c).ToString("x4"));
        else sb.Append(c);
      }
      return sb.Append('"').ToString();
    }

    // ---------- JSON request (UnityEngine.JsonUtility — bounded, no eval) ----------

    [Serializable]
    private class RequestArgs
    {
      public string folder;
      public string def;
      public string key;
      public string path;
    }

    [Serializable]
#pragma warning disable 649 // fields are populated by JsonUtility, not by code
    private class Request
    {
      public int protocol_version;
      public string request_id;
      public string run_id;
      public string op;
      public RequestArgs args;
    }
#pragma warning restore 649

    private class ParsedCommand
    {
      public string FileName;
      public Request Req;
      public bool FromLegacyLineFormat;
      public string ResponseJsonFallback; // envelope-level rejection, written without dispatch
    }

    // One unit of work. Pending screenshot completes on a second pass.
    private class Job
    {
      public ParsedCommand Cmd;
      public string ResponseJson;        // final envelope (written when set)
      public bool PendingCapture;        // watcher must poll for the file first
      public string CapturePath;
      public int FrameAtRequest;
      public int Frame;                  // Time.frameCount — valid on the main thread only
      public bool MainThread;            // honest telemetry: where did this op execute
    }

    // Dispatch policy (v1 lesson of both the SIGTRAP and the frozen queue):
    //  - Pure C# reads (DefDatabase/LanguageDatabase lookups, envelope checks)
    //    answer INSTANTLY on the bridge thread. At the occluded main menu the
    //    long-event queue drains frame-by-frame — minutes for a read; reads
    //    must not wait for it.
    //  - Unity-API ops (select_language, screenshot, quit; research_tab once a
    //    colony exists) go to the MAIN thread: ExecuteWhenFinished when Playing
    //    (golden-acceptance-proven: main thread, end of frame), QueueLongEvent
    //    at Entry (slow but correct — the conformance probe gates on it).
    private static bool DispatchInline(string op)
    {
      switch (op)
      {
        case "capabilities":
        case "state":
        case "keyed":
        case "read_def":
          return true;
        case "research_tab":
          // The Entry-state precondition error touches no Unity API — answer inline.
          return Current.ProgramState != ProgramState.Playing;
        case "select_language":
        case "screenshot":
        case "quit":
          return false; // Unity API — main thread
        default:
          return true;  // unknown op = envelope-level refusal, no Unity API
      }
    }

    static RuntimeBridge()
    {
      try
      {
        Directory.CreateDirectory(CommandsDir);
        Directory.CreateDirectory(ResultsDir);
        QuarantineStaleFiles();
        // Test-only liveness guarantee: without Run-In-Background the player
        // loop stops when the window is unfocused/occluded, and every
        // main-thread dispatch (capture, quit) starves (live lesson of runs
        // 071216/071750). The sandbox Prefs set it too; this enforces it.
        Application.runInBackground = true;
        Log.Message("[RimLoc-RuntimeBridge] v" + BridgeVersion + " up; run_id=" + RunId + "; QA dir=" + QaDir +
                    "; evidence_root=" + EffectiveEvidenceRoot() +
                    "; dispatch=" + (UnityCtx != null ? "sync-context" : "long-event-queue") +
                    "; runInBackground=" + Application.runInBackground);
        WriteStateFile("ready");
        new Thread(CommandLoop) { IsBackground = true, Name = "RimLocRuntimeBridge" }.Start();
      }
      catch (Exception e)
      {
        Log.Error("[RimLoc-RuntimeBridge] init failed: " + e);
      }
    }

    // §9 run identity: files left by an earlier run in the same sandbox must
    // never execute against (or masquerade as) this run.
    private static void QuarantineStaleFiles()
    {
      string stamp = DateTime.UtcNow.ToString("yyyyMMdd-HHmmss");
      foreach (string dir in new[] { CommandsDir, ResultsDir })
        foreach (string f in Directory.GetFiles(dir))
        {
          string rel = Path.GetFileName(f);
          string destDir = Path.Combine(QaDir, "stale", stamp);
          Directory.CreateDirectory(destDir);
          try { File.Move(f, Path.Combine(destDir, rel)); }
          catch (Exception e) { Log.Warning("[RimLoc-RuntimeBridge] quarantine skip " + rel + ": " + e.Message); }
        }
    }

    private static string EffectiveEvidenceRoot()
    {
      string r = EvidenceRoot;
      if (r != null) return r;
      return QaDir;
    }

    // ---------- Bridge thread: poll commands, dispatch main-thread jobs ----------

    private static void CommandLoop()
    {
      while (true)
      {
        try
        {
          foreach (string cmdFile in Directory.GetFiles(CommandsDir))
          {
            ParsedCommand parsed = ReadCommand(cmdFile);
            if (parsed == null) continue; // unknown extension: ignore silently
            try { File.Delete(cmdFile); } catch (Exception e) { Log.Warning("[RimLoc-RuntimeBridge] rm cmd: " + e.Message); }
            if (parsed.Req == null || parsed.ResponseJsonFallback != null)
            {
              // Envelope-level rejection (bad json/protocol/run_id) — respond now.
              File.WriteAllText(Path.Combine(ResultsDir, parsed.FileName), parsed.ResponseJsonFallback);
              continue;
            }
            var job = new Job { Cmd = parsed };
            if (DispatchInline(parsed.Req.op))
            {
              ExecuteJob(job, false);
              if (!job.PendingCapture && job.ResponseJson != null)
                WriteResult(parsed.FileName, job.ResponseJson);
            }
            else
            {
              Action dispatch = () =>
              {
                ExecuteJob(job, true);
                if (!job.PendingCapture && job.ResponseJson != null)
                  WriteResult(parsed.FileName, job.ResponseJson);
              };
              if (UnityCtx != null)
                UnityCtx.Post(_ => dispatch(), null);
              else
                LongEventHandler.QueueLongEvent(dispatch, "rimloc-bridge-op", false, null);
            }
            if (parsed.Req.op == "screenshot")
            {
              string name = parsed.FileName;
              // Wait for the main-thread job to schedule the capture (bounded),
              // then poll for the rendered file.
              var sw = Stopwatch.StartNew();
              while (!job.PendingCapture && sw.ElapsedMilliseconds < 10000) Thread.Sleep(100);
              if (job.PendingCapture) FinishCapture(job, name);
            }
          }
        }
        catch (Exception e) { Log.Warning("[RimLoc-RuntimeBridge] loop: " + e); }
        Thread.Sleep(250);
      }
    }

    // ---------- Manual bounded args extraction ----------
    // Unity's JsonUtility proved unreliable for the nested args object (fields
    // came back null in live run 065258). We parse the FOUR known string keys
    // ourselves: bounded, no reflection, no eval, no dynamic type loading.

    private static void ExtractArgs(string json, RequestArgs args)
    {
      args.folder = ExtractStringArg(json, "folder");
      args.def = ExtractStringArg(json, "def");
      args.key = ExtractStringArg(json, "key");
      args.path = ExtractStringArg(json, "path");
    }

    private static string ExtractStringArg(string json, string name)
    {
      string pat = "\"" + name + "\"";
      int i = json.IndexOf(pat, StringComparison.Ordinal);
      if (i < 0) return null;
      int p = i + pat.Length;
      while (p < json.Length && char.IsWhiteSpace(json[p])) p++;
      if (p >= json.Length || json[p] != ':') return null;
      p++;
      while (p < json.Length && char.IsWhiteSpace(json[p])) p++;
      if (p >= json.Length || json[p] != '"') return null; // absent / null / non-string
      var sb = new StringBuilder();
      p++;
      while (p < json.Length)
      {
        char c = json[p];
        if (c == '"') return sb.ToString();
        if (c == '\\')
        {
          p++;
          if (p >= json.Length) return null;
          char e = json[p];
          if (e == '"') sb.Append('"');
          else if (e == '\\') sb.Append('\\');
          else if (e == '/') sb.Append('/');
          else if (e == 'n') sb.Append('\n');
          else if (e == 't') sb.Append('\t');
          else if (e == 'r') sb.Append('\r');
          else if (e == 'u' && p + 4 < json.Length)
          {
            sb.Append((char)Convert.ToInt32(json.Substring(p + 1, 4), 16));
            p += 4;
          }
          else return null; // unknown escape — reject rather than guess
        }
        else sb.Append(c);
        p++;
      }
      return null; // unterminated string
    }

    private static ParsedCommand ReadCommand(string path)
    {
      var name = Path.GetFileName(path);
      string ext = Path.GetExtension(name).ToLowerInvariant();
      if (ext != ".json" && ext != ".txt") return null;
      long len = new FileInfo(path).Length;
      if (len > MaxRequestBytes)
        return Reject(name, "BAD_REQUEST", "request exceeds " + MaxRequestBytes + " bytes", null);
      string text;
      try { text = File.ReadAllText(path); }
      catch (Exception e) { return Reject(name, "BAD_REQUEST", "unreadable: " + e.Message, null); }

      if (ext == ".txt")
      {
        // Legacy v0.3 line format — accepted through the transition wave only.
        var req = new Request { protocol_version = 0, request_id = "legacy-" + Path.GetFileNameWithoutExtension(name) };
        var args = new RequestArgs();
        foreach (string line in text.Split('\n'))
        {
          int i = line.IndexOf('=');
          if (i <= 0) continue;
          string k = line.Substring(0, i).Trim();
          string v = line.Substring(i + 1).Trim();
          if (k == "op") req.op = v;
          else if (k == "folder") args.folder = v;
          else if (k == "def") args.def = v;
          else if (k == "key") args.key = v;
          else if (k == "path") args.path = v;
        }
        if (string.IsNullOrEmpty(req.op))
          return Reject(name, "BAD_REQUEST", "legacy command has no op", null);
        req.args = args;
        return new ParsedCommand { FileName = name, Req = req, FromLegacyLineFormat = true };
      }

      Request r;
      try { r = JsonUtility.FromJson<Request>(text); }
      catch (Exception e) { return Reject(name, "BAD_REQUEST", "malformed JSON: " + e.Message, null); }
      if (r == null || string.IsNullOrEmpty(r.op))
        return Reject(name, "BAD_REQUEST", "missing op", null);
      // JsonUtility is authoritative for the flat envelope fields only; args
      // are re-parsed manually (nested-object deserialization proved flaky).
      if (r.args == null) r.args = new RequestArgs();
      ExtractArgs(text, r.args);
      var partial = new ParsedCommand { FileName = name, Req = r };
      if (r.protocol_version != ProtocolVersion)
        return RejectOf(partial, "PROTOCOL_VERSION_UNSUPPORTED",
            "bridge speaks " + ProtocolVersion + ", request says " + r.protocol_version, null);
      if (!string.IsNullOrEmpty(r.run_id) && r.run_id != RunId)
        return RejectOf(partial, "RUN_ID_MISMATCH", "this bridge is " + RunId, null);
      return partial;
    }

    private static ParsedCommand Reject(string fileName, string code, string message, Dictionary<string, string> detail)
    {
      // Request identity unknown (unreadable/unparseable) — honest "req-unknown".
      var p = new ParsedCommand { FileName = fileName };
      p.ResponseJsonFallback = EnvelopeRaw(new Job(), false, null, ErrorJson(code, message, detail), null, null);
      return p;
    }

    private static ParsedCommand RejectOf(ParsedCommand p, string code, string message, Dictionary<string, string> detail)
    {
      // Parsed enough to echo request_id — correlation stays intact.
      p.ResponseJsonFallback = EnvelopeRaw(new Job { Cmd = p }, false, null, ErrorJson(code, message, detail), null, null);
      return p;
    }

    // ---------- Execute one op (bridge thread when inline, main thread when dispatched) ----------

    private static void ExecuteJob(Job job, bool mainThread)
    {
      job.MainThread = mainThread;
      job.Frame = mainThread ? Time.frameCount : 0;
      int frame = job.Frame;
      var req = job.Cmd.Req;
      try
      {
        switch (req.op)
        {
          case "capabilities":
            job.ResponseJson = Ok(job, CapabilitiesJson());
            return;
          case "state":
          {
            string state = StateJson("on_demand", frame);
            WriteStateFile("on_demand");
            job.ResponseJson = RawResultOk(job, state);
            return;
          }
          case "select_language":
          {
            string folder = Arg(req, "folder");
            LoadedLanguage lang = LanguageDatabase.AllLoadedLanguages.FirstOrDefault(l => l.folderName == folder);
            if (lang == null)
            {
              var loaded = LanguageDatabase.AllLoadedLanguages.Select(l => l.folderName).ToArray();
              job.ResponseJson = Fail(job, "LANGUAGE_NOT_READY",
                  "language not loaded: " + folder, new Dictionary<string, string> { { "loaded", string.Join("|", loaded) } });
              return;
            }
            LanguageDatabase.SelectLanguage(lang);
            stateRevision++;
            string state = StateJson("after_select_language", Time.frameCount);
            WriteStateFile("after_select_language", job.Frame);
            job.ResponseJson = RawResultOk(job, "{\"selected\":" + Esc(folder) + ",\"state\":" + state + "}");
            return;
          }
          case "research_tab":
          {
            // Semantic preconditions — NEVER the raw InvalidCastException of v0.3.
            if (Current.ProgramState != ProgramState.Playing)
            {
              job.ResponseJson = Fail(job, "WRONG_GAME_STATE",
                  "research tab exists only inside a colony",
                  new Dictionary<string, string> { { "program_state", Current.ProgramState.ToString() } });
              return;
            }
            if (Find.CurrentMap == null)
            {
              job.ResponseJson = Fail(job, "NOT_IN_COLONY",
                  "no colony map loaded — semantic tab root is not available", null);
              return;
            }
            MainButtonDef tab = DefDatabase<MainButtonDef>.GetNamed("Research", false);
            if (tab == null)
            {
              var all = string.Join("|", DefDatabase<MainButtonDef>.AllDefs.Select(t => t.defName).ToArray());
              job.ResponseJson = Fail(job, "TARGET_DEF_NOT_FOUND",
                  "Research MainButtonDef not registered",
                  new Dictionary<string, string> { { "buttons", all } });
              return;
            }
            Find.MainTabsRoot.SetCurrentTab(tab);
            stateRevision++;
            job.ResponseJson = Ok(job, "{\"tab\":" + Esc(tab.defName) + ",\"state_revision\":" + stateRevision + "}");
            return;
          }
          case "read_def":
          {
            string defName = Arg(req, "def");
            ResearchProjectDef def = DefDatabase<ResearchProjectDef>.GetNamedSilentFail(defName);
            if (def == null)
            {
              job.ResponseJson = Fail(job, "TARGET_DEF_NOT_FOUND",
                  "ResearchProjectDef not found: " + defName, null);
              return;
            }
            job.ResponseJson = Ok(job,
                "{\"def\":" + Esc(defName) + ",\"label\":" + Esc(def.LabelCap.RawText) +
                ",\"description\":" + Esc(def.description) + "}");
            return;
          }
          case "keyed":
          {
            string key = Arg(req, "key");
            TaggedString ts = key.Translate();
            job.ResponseJson = Ok(job, "{\"key\":" + Esc(key) + ",\"value\":" + Esc(ts.RawText) + "}");
            return;
          }
          case "screenshot":
          {
            string rawPath = Arg(req, "path");
            if (string.IsNullOrEmpty(rawPath))
            {
              job.ResponseJson = Fail(job, "BAD_REQUEST",
                  "screenshot requires args.path (inside the evidence root)", null);
              return;
            }
            string target = FenceCapturePath(rawPath);
            if (target == null)
            {
              job.ResponseJson = Fail(job, "EVIDENCE_ROOT_ESCAPE",
                  "screenshot path must stay inside " + EffectiveEvidenceRoot(),
                  new Dictionary<string, string> { { "evidence_root", EffectiveEvidenceRoot() } });
              return;
            }
            if (File.Exists(target))
            {
              try { File.Delete(target); } catch (Exception e) { Log.Warning("[RimLoc-RuntimeBridge] stale capture rm: " + e.Message); }
            }
            job.CapturePath = target;
            job.FrameAtRequest = frame;
            job.PendingCapture = true;
            // ScreenCapture writes no intermediate directories — create them,
            // otherwise the capture silently never lands on disk.
            string dir = Path.GetDirectoryName(target);
            if (!string.IsNullOrEmpty(dir)) Directory.CreateDirectory(dir);
            // Least invasive test-only render request: the capture itself is
            // scheduled to the END of the current frame — requesting a
            // screenshot forces exactly one more presented frame, no focus,
            // no window manipulation.
            ScreenCapture.CaptureScreenshot(target);
            return;
          }
          case "quit":
            Log.Message("[RimLoc-RuntimeBridge] quit requested; run " + RunId + " ending");
            job.ResponseJson = Ok(job, "{\"quitting\":true}");
            Application.Quit();
            return;
          default:
            job.ResponseJson = Fail(job, "CAPABILITY_UNAVAILABLE",
                "unknown op: " + req.op,
                new Dictionary<string, string> { { "ops", "capabilities state select_language research_tab read_def keyed screenshot quit" } });
            return;
        }
      }
      catch (Exception e)
      {
        // Diagnostic detail to the log only; contract stays semantic (§4).
        Log.Warning("[RimLoc-RuntimeBridge] op " + req.op + " failed: " + e);
        job.ResponseJson = Fail(job, "INTERNAL",
            e.GetType().Name + " (see game log for detail)", null);
      }
    }

    // Direct field access only — deliberate absence of reflection (§9).
    private static string Arg(Request req, string name)
    {
      if (req.args == null) return null;
      switch (name)
      {
        case "folder": return req.args.folder;
        case "def": return req.args.def;
        case "key": return req.args.key;
        case "path": return req.args.path;
        default: return null;
      }
    }

    // ---------- Screenshot completion (frame freshness, §5) ----------

    private static void FinishCapture(Job job, string resultFileName)
    {
      var sw = Stopwatch.StartNew();
      while (sw.ElapsedMilliseconds < CaptureTimeoutMs)
      {
        if (File.Exists(job.CapturePath) && new FileInfo(job.CapturePath).Length > 0) break;
        Thread.Sleep(100);
      }
      bool fileOk = File.Exists(job.CapturePath) && new FileInfo(job.CapturePath).Length > 0;
      long size = fileOk ? new FileInfo(job.CapturePath).Length : 0;
      Action stampCapture = () =>
      {
        job.MainThread = true;
        int frameAtPresent = Time.frameCount;
        job.Frame = frameAtPresent;
        bool advanced = frameAtPresent > job.FrameAtRequest;
        // Frame advanced + file written by the render pipeline ⇒ the capture
        // corresponds to a state at or after the request. If rendering was
        // throttled to a stop, frame_at_present == frame_at_request and the
        // consumer must treat the frame as stale.
        string freshness = "{\"frame_at_request\":" + job.FrameAtRequest +
                           ",\"frame_at_present\":" + frameAtPresent +
                           ",\"frame_advanced\":" + (advanced ? "true" : "false") +
                           ",\"state_revision\":" + stateRevision +
                           ",\"stale\":" + (advanced ? "false" : "true") + "}";
        if (!fileOk)
        {
          job.ResponseJson = EnvelopeRaw(job, false, null,
              ErrorJson("CAPTURE_TIMEOUT",
                  "capture file did not appear within " + CaptureTimeoutMs + " ms",
                  new Dictionary<string, string> { { "path", job.CapturePath } }),
              Telemetry(job), ",\"freshness\":" + freshness);
        }
        else
        {
          job.ResponseJson = RawResultOk(job,
              "{\"path\":" + Esc(job.CapturePath) + ",\"size_bytes\":" + size + ",\"freshness\":" + freshness + "}");
        }
        WriteResult(resultFileName, job.ResponseJson);
        job.ResponseJson = null; // written; CommandLoop must not double-write
      };
      if (UnityCtx != null) UnityCtx.Post(_ => stampCapture(), null);
      else LongEventHandler.QueueLongEvent(stampCapture, "rimloc-bridge-capture", false, null);
    }

    // ---------- Envelope builders ----------

    private static string Ok(Job job, string resultJson)
    {
      return RawResultOk(job, resultJson);
    }

    private static string RawResultOk(Job job, string resultJson)
    {
      return EnvelopeRaw(job, true, resultJson, null, Telemetry(job));
    }

    private static string Fail(Job job, string code, string message, Dictionary<string, string> detail)
    {
      return EnvelopeRaw(job, false, null, ErrorJson(code, message, detail), Telemetry(job));
    }

    private static string EnvelopeRaw(Job job, bool ok, string resultJson, string errorJson, string telemetry)
    {
      return EnvelopeRaw(job, ok, resultJson, errorJson, telemetry, null);
    }

    private static string EnvelopeRaw(Job job, bool ok, string resultJson, string errorJson, string telemetry,
        string extraFieldsJson)
    {
      var sb = new StringBuilder("{\"protocol_version\":").Append(ProtocolVersion);
      sb.Append(",\"request_id\":").Append(Esc(JobRequestId(job)));
      sb.Append(",\"run_id\":").Append(Esc(RunId));
      sb.Append(",\"ok\":").Append(ok ? "true" : "false");
      sb.Append(",\"result\":").Append(resultJson ?? "null");
      sb.Append(",\"error\":").Append(errorJson ?? "null");
      sb.Append(",\"telemetry\":").Append(telemetry ?? "null");
      if (!string.IsNullOrEmpty(extraFieldsJson)) sb.Append(extraFieldsJson);
      sb.Append("}");
      return sb.ToString();
    }

    private static string JobRequestId(Job job)
    {
      if (job?.Cmd?.Req == null) return "req-unknown";
      string id = job.Cmd.Req.request_id;
      return string.IsNullOrEmpty(id)
          ? (job.Cmd.FromLegacyLineFormat ? "legacy-" + job.Cmd.FileName : "req-unknown")
          : id;
    }

    private static string Telemetry(Job job)
    {
      return "{\"frame\":" + (job.MainThread ? job.Frame.ToString() : "null") +
             ",\"main_thread\":" + (job.MainThread ? "true" : "false") +
             ",\"state_revision\":" + stateRevision +
             ",\"wall_utc\":\"" + DateTime.UtcNow.ToString("o") + "\"}";
    }

    private static string ErrorJson(string code, string message, Dictionary<string, string> detail)
    {
      var sb = new StringBuilder("{\"code\":").Append(Esc(code));
      sb.Append(",\"message\":").Append(Esc(message));
      if (detail == null || detail.Count == 0) sb.Append(",\"detail\":null");
      else
      {
        sb.Append(",\"detail\":{");
        bool first = true;
        foreach (var kv in detail)
        {
          if (!first) sb.Append(",");
          sb.Append(Esc(kv.Key)).Append(":").Append(Esc(kv.Value));
          first = false;
        }
        sb.Append("}");
      }
      return sb.Append("}").ToString();
    }

    // ---------- State snapshot (language model: folder identity vs display names) ----------

    private static string StateJson(string phase, int frame)
    {
      LoadedLanguage lang = LanguageDatabase.activeLanguage;
      var langs = string.Join(",",
          LanguageDatabase.AllLoadedLanguages.Select(l =>
              "{\"folder\":" + Esc(l.folderName) + // canonical identity (§6)
              ",\"native\":" + Esc(l.info != null ? l.info.friendlyNameNative : null) + // display
              ",\"english\":" + Esc(l.info != null ? l.info.friendlyNameEnglish : null) + "}").ToArray()); // display
      var mods = string.Join(",",
          ModLister.AllInstalledMods.Select(m =>
              "{\"package_id\":" + Esc(m.PackageId) + ",\"active\":" + (m.Active ? "true" : "false") + "}").ToArray());
      ResearchProjectDef hw = DefDatabase<ResearchProjectDef>.GetNamedSilentFail("VWE_HeavyWeapons");
      return "{\"phase\":" + Esc(phase) +
             ",\"bridge_version\":" + Esc(BridgeVersion) +
             ",\"protocol_version\":" + ProtocolVersion +
             ",\"run_id\":" + Esc(RunId) +
             ",\"game_version\":" + Esc(VersionControl.CurrentVersionString) +
             ",\"program_state\":" + Esc(Current.ProgramState.ToString()) +
             ",\"active_language\":{\"folder\":" + Esc(lang != null ? lang.folderName : null) +
             ",\"native\":" + Esc(lang != null && lang.info != null ? lang.info.friendlyNameNative : null) +
             ",\"english\":" + Esc(lang != null && lang.info != null ? lang.info.friendlyNameEnglish : null) + "}" +
             ",\"languages\":[" + langs + "]" +
             ",\"mods\":[" + mods + "]" +
             ",\"research_count\":" + DefDatabase<ResearchProjectDef>.AllDefs.Count() +
             ",\"state_revision\":" + stateRevision +
             ",\"frame\":" + frame +
             // Golden T6 markers — identity anchors of the golden acceptance (§2, never mutated).
             ",\"marker_research_description\":" + Esc(hw != null ? hw.description : null) +
             ",\"marker_keyed\":" + Esc("VWE_ShotRemaining".Translate().RawText) + "}";
    }

    private static void WriteStateFile(string phase, int frame = 0)
    {
      try { File.WriteAllText(Path.Combine(QaDir, "state.json"), StateJson(phase, Time.frameCount)); }
      catch (Exception e) { Log.Warning("[RimLoc-RuntimeBridge] state.json: " + e.Message); }
    }

    // ---------- Capability manifest (§8; Fabric capability.schema.json entries) ----------

    private static string CapabilitiesJson()
    {
      const string evidence = "RimLoc-evidence/p0-incident-20261001/t6-background-acceptance/ACCEPTANCE.md (2026-10-03)";
      return "{\"adapter\":{\"id\":\"rimworld-runtime-bridge\",\"version\":\"" + BridgeVersion + "\"" +
             ",\"fabric_path\":\"AI-OS App Control Fabric → RimWorld Adapter → RimWorld Runtime Bridge\"" +
             ",\"target\":\"RimWorld 1.6 (Unity Mono player)\"" +
             ",\"transport\":\"file channel: savedata/QA commands→results\"" +
             ",\"deployment\":\"test-only, isolated clone profile\"" +
             ",\"golden_acceptance\":\"T6 REAL GAME + T6 BACKGROUND AUTONOMOUS = PASS 2026-10-03\"}" +
             ",\"protocol\":{\"version\":" + ProtocolVersion +
             ",\"discovery_op\":\"capabilities\"" +
             ",\"legacy_line_format\":\"accepted, deprecated\"}" +
             ",\"capabilities\":[" +
             "{\"name\":\"runtime.ready\",\"class\":\"observe\",\"proven\":true,\"evidence\":\"" + evidence + "\"}," +
             "{\"name\":\"runtime.state\",\"class\":\"observe\",\"proven\":true,\"evidence\":\"" + evidence + "\"}," +
             "{\"name\":\"localization.keyed\",\"class\":\"observe\",\"proven\":true,\"evidence\":\"" + evidence + "\"}," +
             "{\"name\":\"localization.def_injected\",\"class\":\"observe\",\"proven\":true,\"evidence\":\"" + evidence + "\"}," +
             "{\"name\":\"ui.research_navigation\",\"class\":\"app\",\"proven\":true,\"evidence\":\"" + evidence + "\"}," +
             "{\"name\":\"capture.game_frame\",\"class\":\"recording\",\"proven\":true,\"evidence\":\"" + evidence + "\"}," +
             "{\"name\":\"capture.frame_freshness\",\"class\":\"recording\",\"proven\":true,\"evidence\":\"" + evidence + " + v1 conformance run\"}," +
             "{\"name\":\"lifecycle.quit\",\"class\":\"app\",\"proven\":true,\"evidence\":\"" + evidence + "\"}" +
             "]" +
             ",\"constraints\":{\"accessibility.semantic_ui\":\"limited\",\"global_input.required\":false,\"foreground_required\":false}" +
             ",\"language_model\":{\"identity\":\"language folder name\",\"display\":[\"friendlyNameNative\",\"friendlyNameEnglish\"]" +
             ",\"claim_inactive_pack_registration\":\"UNCONFIRMED\"}" +
             ",\"ops\":[\"capabilities\",\"state\",\"select_language\",\"research_tab\",\"read_def\",\"keyed\",\"screenshot\",\"quit\"]}";
    }

    // ---------- Path fence (§9: no arbitrary filesystem access) ----------

    private static string FenceCapturePath(string raw)
    {
      if (string.IsNullOrEmpty(raw)) return null;
      string root = Path.GetFullPath(EffectiveEvidenceRoot());
      string candidate = Path.IsPathRooted(raw) ? raw : Path.Combine(root, raw);
      string full;
      try { full = Path.GetFullPath(candidate); }
      catch { return null; }
      if (!full.StartsWith(root + Path.DirectorySeparatorChar, StringComparison.Ordinal)) return null;
      return full;
    }

    private static void WriteResult(string name, string json)
    {
      try { File.WriteAllText(Path.Combine(ResultsDir, name), json); }
      catch (Exception e) { Log.Warning("[RimLoc-RuntimeBridge] result write " + name + ": " + e.Message); }
    }
  }
}
