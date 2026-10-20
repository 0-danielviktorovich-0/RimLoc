import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const script = fileURLToPath(new URL('../hooks/workflow-guard.mjs', import.meta.url));
const workspace = mkdtempSync(join(tmpdir(), 'rimloc-hook-fixture-'));
const data = join(workspace, 'private-plugin-data');
mkdirSync(join(workspace, 'docs/development'), { recursive: true });
mkdirSync(join(workspace, 'tools/zcode/rimloc-orchestrator/skills/rimloc-release-orchestrator'), { recursive: true });
writeFileSync(join(workspace, 'AGENTS.md'), '# RimLoc\n');
writeFileSync(join(workspace, 'docs/development/ZCODE_WORKFLOW_RUNTIME.md'), '# Runtime\n');
writeFileSync(join(workspace, 'tools/zcode/rimloc-orchestrator/skills/rimloc-release-orchestrator/SKILL.md'), '# Skill\n');

function hook(sessionId, event, extra = {}, cwd = workspace) {
  const r = spawnSync(process.execPath, [script], {
    input: JSON.stringify({
      session_id: sessionId,
      hook_event_name: event,
      cwd,
      ...extra
    }) + '\n',
    encoding: 'utf8',
    env: { ...process.env, ZCODE_PLUGIN_DATA: data }
  });
  assert.equal(r.status, 0, r.stderr);
  return r.stdout ? JSON.parse(r.stdout) : null;
}

test('SessionStart injects actual workflow mandate in RimLoc workspace', () => {
  const result = hook('start-test', 'SessionStart', { source: 'startup' });
  assert.match(result.hookSpecificOutput.additionalContext, /CreateWorkflow/);
  assert.match(result.hookSpecificOutput.additionalContext, /WORKFLOW_UNAVAILABLE/);
});

test('does nothing in a different workspace', () => {
  const elsewhere = mkdtempSync(join(tmpdir(), 'other-project-'));
  assert.equal(hook('other', 'SessionStart', {}, elsewhere), null);
});

test('does not block a trivial one-file fix', () => {
  assert.equal(hook('small', 'UserPromptSubmit', { prompt: 'Исправь опечатку в одной строке файла README.' }), null);
  assert.equal(hook('small', 'Stop', { last_assistant_message: 'Опечатку исправил.' }), null);
});

test('multi-subsystem ZCode campaign must have observed native workflow or precise fallback', () => {
  const injected = hook('campaign-a', 'UserPromptSubmit', {
    prompt: 'Разработай полностью несколько подсистем и продолжай автономно до релиза параллельными агентами.'
  });
  assert.match(injected.hookSpecificOutput.additionalContext, /CreateWorkflow/);
  const first = hook('campaign-a', 'Stop', { last_assistant_message: 'Все задачи выполнены.' });
  assert.equal(first.decision, 'block');
  assert.match(first.reason, /no observed native/);
  const second = hook('campaign-a', 'Stop', { last_assistant_message: 'Вот план.' });
  assert.equal(second.decision, 'block');
  const third = hook('campaign-a', 'Stop', { last_assistant_message: 'Вот план.' });
  assert.equal(third, null, 'bounded feedback; no infinite stop loop');
});

test('native CreateWorkflow post-tool receipt is accepted', () => {
  hook('campaign-b', 'UserPromptSubmit', {
    prompt: 'Реализуй полностью интеграцию MCP и Skills, используй workflow и параллельных агентов.'
  });
  hook('campaign-b', 'PostToolUse', {
    tool_name: 'CreateWorkflow',
    tool_response: { run_id: 'run-observed-123', status: 'running' }
  });
  assert.equal(hook('campaign-b', 'Stop', { last_assistant_message: 'WORKFLOW_RUN: run-observed-123' }), null);
});

test('typed tool error is not accepted as a workflow receipt', () => {
  hook('campaign-error', 'UserPromptSubmit', {
    prompt: 'Разработай полностью архитектуру с несколькими параллельными направлениями.'
  });
  hook('campaign-error', 'PostToolUse', {
    tool_name: 'CreateWorkflow',
    tool_response: { error: 'Not available' }
  });
  const stop = hook('campaign-error', 'Stop', { last_assistant_message: 'Created run.' });
  assert.equal(stop.decision, 'block');
});

test('honest unavailability after tool probing allows a bounded fallback', () => {
  hook('campaign-c', 'UserPromptSubmit', {
    prompt: 'Продолжи разработку всей системы автономно в параллельных направлениях.'
  });
  const stop = hook('campaign-c', 'Stop', {
    last_assistant_message: 'WORKFLOW_UNAVAILABLE: CreateWorkflow missing from actual ZCode tool catalog; using a sequential bounded fallback.'
  });
  assert.equal(stop, null);
});

test('a prior workflow can be reused when explicitly identified', () => {
  hook('campaign-d', 'UserPromptSubmit', {
    prompt: 'Разработай полностью проект в параллельных направлениях.'
  });
  hook('campaign-d', 'PostToolUse', {
    tool_name: 'CreateWorkflow',
    tool_response: { run_id: 'run-previous', status: 'running' }
  });
  hook('campaign-d', 'UserPromptSubmit', {
    prompt: 'Продолжи автономно весь проект через прежний workflow.'
  });
  assert.equal(hook('campaign-d', 'Stop', {
    last_assistant_message: 'WORKFLOW_EXISTING: run-previous; verified active in ZCode dashboard.'
  }), null);
});

test('state persists only flags, not user prompt or tool output', () => {
  hook('no-secrets', 'UserPromptSubmit', {
    prompt: 'Разработай полностью интеграцию параллельно. SECRET_SENTINEL_SHOULD_NOT_PERSIST'
  });
  hook('no-secrets', 'PostToolUse', {
    tool_name: 'CreateWorkflow',
    tool_response: { run_id: 'xyz', sensitive: 'SECRET_TOOL_SENTINEL' }
  });
  const { readdirSync } = await import('node:fs');
  const files = readdirSync(join(data, 'workflow-guard-state'));
  const contents = files.map(f => readFileSync(join(data, 'workflow-guard-state', f), 'utf8')).join('\n');
  assert.ok(!contents.includes('SECRET_SENTINEL_SHOULD_NOT_PERSIST'));
  assert.ok(!contents.includes('SECRET_TOOL_SENTINEL'));
});
