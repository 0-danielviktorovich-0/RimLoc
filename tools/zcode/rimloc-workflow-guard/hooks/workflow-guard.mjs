/**
 * ZCode RimLoc workflow guard, v0.1.0.
 * Hook subprocess, not a tool or a substitute for CreateWorkflow.
 * Only activates when cwd is inside the RimLoc checkout.
 * No network, shell, credential access, or prompt/transcript retention.
 */
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';

const MAX_INPUT = 128 * 1024;
let raw = '';
for await (const chunk of process.stdin) {
  raw += chunk;
  if (raw.length > MAX_INPUT) process.exit(0);
}

let input;
try { input = JSON.parse(raw); } catch { process.exit(0); }

function repoRoot(cwd) {
  let current = resolve(cwd);
  for (let i = 0; i < 14; i++) {
    if (existsSync(join(current, 'AGENTS.md')) &&
        existsSync(join(current, 'docs/development/ZCODE_WORKFLOW_RUNTIME.md')) &&
        existsSync(join(current, 'tools/zcode/rimloc-orchestrator/skills/rimloc-release-orchestrator/SKILL.md'))) {
      return current;
    }
    const parent = dirname(current);
    if (parent === current) break;
    current = parent;
  }
  return null;
}

const root = repoRoot(input.cwd || process.cwd());
if (!root) process.exit(0);

const event = input.hook_event_name || input.hookEventName;
const sessionId = String(input.session_id || input.sessionId || '');
const stateDir = process.env.ZCODE_PLUGIN_DATA
  ? join(process.env.ZCODE_PLUGIN_DATA, 'workflow-guard-state')
  : null;
const statePath = stateDir && sessionId
  ? join(stateDir, createHash('sha256').update(sessionId).digest('hex') + '.json')
  : null;

function readState() {
  if (!statePath) return {};
  try { return JSON.parse(readFileSync(statePath, 'utf8')); } catch { return {}; }
}

function saveState(value) {
  if (!statePath) return;
  try {
    mkdirSync(stateDir, { recursive: true, mode: 0o700 });
    const tmp = statePath + '.tmp-' + process.pid;
    writeFileSync(tmp, JSON.stringify(value), { mode: 0o600 });
    renameSync(tmp, statePath);
  } catch (err) {
    process.stderr.write('[rimloc-workflow-guard] Cannot persist hook state: ' + err.message + '\n');
  }
}

function context(message) {
  process.stdout.write(JSON.stringify({
    hookSpecificOutput: { hookEventName: event, additionalContext: message }
  }));
}

const workflowInstruction = [
  'RimLoc ZCode project policy (auto-injected; do NOT request this from the owner in each prompt):',
  'For tasks spanning 2+ subsystems, 3+ deliverables, architecture, security, release or explicit parallelism, use the ACTUAL native ZCode CreateWorkflow / ResumeWorkflowRun / AmendWorkflow rather than serial single-agent work.',
  'Read AGENTS.md, tools/zcode/rimloc-orchestrator/skills/rimloc-release-orchestrator/SKILL.md and docs/development/ZCODE_WORKFLOW_RUNTIME.md.',
  'Check tool availability; launch or resume a bounded workflow with named lanes, non-conflicting worktrees, deterministic tests and progress receipts BEFORE describing the campaign as underway.',
  'If those tools are inaccessible, state WORKFLOW_UNAVAILABLE:<specific tool/probe error> and use documented bounded fallback; never invent a run ID.',
  'At campaign completion provide WORKFLOW_RUN:<actual run ID or real observed tool receipt> or WORKFLOW_EXISTING:<verified running run ID> or explicit WORKFLOW_UNAVAILABLE:<reason>.',
  'Trivial one-file fixes do NOT require CreateWorkflow. Do not block valid owner-only release/credentials decisions or bypass native permissions.'
].join('\n');

function isCampaign(prompt) {
  const p = String(prompt || '').slice(0, 60000);
  const action = /(?:\b(?:implement|build|ship|fix|continue|finish|develop|audit|integrate|review|launch)\b|(?:разработ|реализ|продолж|додел|исправ|запус|внедр|интегрир|провед|провер|тестир|аудит|созда|доработ))/i.test(p);
  const scope = /(?:\b(?:workflow|multi.agent|multi.lane|parallel|autonomous|release|architecture|security|entire|full project|multiple features)\b|(?:воркфло|параллел|субагент|автоном|полностью|до релиза|всего проекта|несколько (?:задач|систем)|архитектур|безопасност|всё приложение))/i.test(p);
  return action && scope;
}

if (event === 'SessionStart') {
  context(workflowInstruction);
} else if (event === 'UserPromptSubmit') {
  const campaign = isCampaign(input.prompt);
  const previous = readState();
  saveState({
    campaign,
    workflowSeenThisPrompt: false,
    feedbackCount: 0,
    previousWorkflowSeen: !!previous.workflowSeenThisPrompt || !!previous.previousWorkflowSeen,
    previousWorkflowAt: previous.workflowAt || null,
    startedAt: Date.now()
  });
  if (campaign) context(workflowInstruction);
} else if (event === 'PostToolUse') {
  const name = String(input.tool_name || input.toolName || '');
  if (/^(?:CreateWorkflow|ResumeWorkflowRun|AmendWorkflow)$/.test(name)) {
    const response = input.tool_response ?? input.toolResponse;
    const serialized = JSON.stringify(response ?? {});
    const seemsFailed = !!(response && typeof response === 'object' &&
      (response.isError === true || response.is_error === true || response.error));
    if (!seemsFailed && response !== undefined) {
      const current = readState();
      current.workflowSeenThisPrompt = true;
      current.previousWorkflowSeen = true;
      current.workflowAt = Date.now();
      current.workflowTool = name;
      // Store no tool input or output: those may contain proprietary paths or text.
      saveState(current);
    }
  }
} else if (event === 'Stop') {
  const state = readState();
  const message = String(input.last_assistant_message || input.lastAssistantMessage || '');
  const hasExplicitFallback = /WORKFLOW_UNAVAILABLE\s*:\s*\S.{8,}/i.test(message);
  const hasExistingReceipt = /WORKFLOW_EXISTING\s*:\s*\S.{2,}/i.test(message) &&
    state.previousWorkflowSeen;
  if (statePath && state.campaign && !state.workflowSeenThisPrompt &&
      !hasExplicitFallback && !hasExistingReceipt && (state.feedbackCount || 0) < 2) {
    state.feedbackCount = (state.feedbackCount || 0) + 1;
    saveState(state);
    process.stdout.write(JSON.stringify({
      decision: 'block',
      reason: [
        'RimLoc multi-lane task has no observed native CreateWorkflow/ResumeWorkflowRun/AmendWorkflow call in this turn.',
        'Inspect actual ZCode tool availability; launch or resume the workflow and record the receipt.',
        'If genuinely unavailable, report WORKFLOW_UNAVAILABLE:<specific tested reason> and bounded fallback.',
        'Do not claim a workflow was executed when only AGENTS.md, a skill, or a plan was read.'
      ].join(' ')
    }));
  }
}
