// Existing translation pack store (W2, mandate "update an existing
// translation"): the reactive state for the dry-run ANALYZE and the guarded
// APPLY of an existing translation pack against the LIVE contract project.
// Same shape as contractops.svelte.ts: the capability store opens the gates
// (Rust reports project_import_existing / project_apply_existing supported);
// demo/fixture projects never reach it — components branch on
// project.source and keep their marked demo flows.
//
// Safety rules surfaced verbatim from the contract:
// - analyze NEVER writes (dry-run; no revision bump);
// - apply moves ONLY the reusable set — existing (human) translations are
//   never overwritten, ambiguous lines are never auto-applied;
// - a revision change between analyze and apply (a manual edit) is caught
//   by expected_revision — the whole apply is refused, nothing is half-done.
import { ContractClientError } from '../client/client';
import { clientInstance } from '../client/instance.svelte';
import type {
  ApplyExistingResponseDto,
  ImportExistingResponseDto
} from '../client/types';
import { project } from './project.svelte';
import { folderForm } from './contractops.svelte';

class ExistingPackStore {
  /** Absolute pack dir chosen via the OS folder picker. */
  existingDir = $state('');
  /** The pack dir the CURRENT analysis was made for (null until analyze
   *  succeeds). Apply is BOUND to it: the analysis a decision is shown
   *  must be the analysis applied — editing the dir after analyze keeps
   *  the numbers visible but disables Apply until a re-analyze. */
  analyzedDir = $state<string | null>(null);
  /** Dry-run analysis result (null until analyze succeeds). */
  analysis = $state<ImportExistingResponseDto | null>(null);
  /** Apply result (null until apply succeeds). */
  applied = $state<ApplyExistingResponseDto | null>(null);
  analyzing = $state(false);
  applying = $state(false);
  /** Last typed contract failure (shown verbatim by the panel). */
  error = $state<string | null>(null);
  /** Which action failed (analyze / apply) — drives the aria target. */
  failedStep = $state<'analyze' | 'apply' | null>(null);

  private cc() {
    return clientInstance.getClient();
  }

  private requireActive(): { projectId: string; epoch: number; locale: string } {
    if (project.source !== 'contract' || !project.contractProjectId) {
      throw new ContractClientError('contract_violation', 'no live contract project is open');
    }
    return {
      projectId: project.contractProjectId,
      epoch: project.contractEpoch,
      locale: folderForm(project.targetLocale)
    };
  }

  reset(): void {
    this.existingDir = '';
    this.analyzedDir = null;
    this.analysis = null;
    this.applied = null;
    this.analyzing = false;
    this.applying = false;
    this.error = null;
    this.failedStep = null;
  }

  /** Clear only the decision state (new dir picked or apply succeeded). */
  private clearDecision(): void {
    this.analysis = null;
    this.applied = null;
    this.error = null;
    this.failedStep = null;
  }

  private fail(e: unknown, step: 'analyze' | 'apply'): void {
    this.error = e instanceof ContractClientError ? `${e.code}: ${e.message}` : String(e);
    this.failedStep = step;
  }

  /** DRY-RUN analyze of the chosen pack dir. Never writes anything. */
  async analyze(): Promise<boolean> {
    let ctx: { projectId: string; epoch: number; locale: string };
    try {
      ctx = this.requireActive();
    } catch (e) {
      this.fail(e, 'analyze');
      return false;
    }
    const dir = this.existingDir.trim();
    if (!dir) {
      this.error = 'existing: choose an existing translation directory first';
      this.failedStep = 'analyze';
      return false;
    }
    this.analyzing = true;
    this.clearDecision();
    try {
      this.analysis = await this.cc().importExisting({
        project_id: ctx.projectId,
        session_epoch: ctx.epoch,
        existing_dir: { path: dir },
        locale: ctx.locale
      });
      this.analyzedDir = dir;
      return true;
    } catch (e) {
      this.fail(e, 'analyze');
      return false;
    } finally {
      this.analyzing = false;
    }
  }

  /** APPLY the reusable set of the analyzed pack (persist-before-ack).
   *  Guarded by the revision the analysis was based on: any manual edit
   *  between analyze and apply is a typed stale_revision refusal. */
  async apply(): Promise<boolean> {
    let ctx: { projectId: string; epoch: number; locale: string };
    try {
      ctx = this.requireActive();
    } catch (e) {
      this.fail(e, 'apply');
      return false;
    }
    const dir = this.existingDir.trim();
    if (!dir) {
      this.error = 'existing: choose an existing translation directory first';
      this.failedStep = 'apply';
      return false;
    }
    this.applying = true;
    this.error = null;
    this.failedStep = null;
    try {
      this.applied = await this.cc().applyExisting({
        project_id: ctx.projectId,
        expected_revision: project.contractAckedRevision || project.contractRevision,
        session_epoch: ctx.epoch,
        existing_dir: { path: dir },
        locale: ctx.locale
      });
      // The backend revision moved (persist-before-ack): adopt the new
      // snapshot so the workspace sees the imported translations.
      await project.refreshContract();
      return true;
    } catch (e) {
      this.fail(e, 'apply');
      return false;
    } finally {
      this.applying = false;
    }
  }
}

export const existingPack = new ExistingPackStore();
