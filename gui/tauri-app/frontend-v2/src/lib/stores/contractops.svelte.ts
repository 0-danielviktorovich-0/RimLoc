// Contract operations store (final night wave): the reactive state for
// validate / export / diagnose over the LIVE contract (RimLocClient). The
// capability store opens the gates (Rust now reports the three supported);
// this store owns the run state, typed errors and the findings→workspace
// mapping. Demo/fixture projects never reach it — components branch on
// project.source and keep their marked demo flows.
import { ContractClientError } from '../client/client';
import { clientInstance } from '../client/instance.svelte';
import type {
  BuildModProjectResponseDto,
  DiagnoseResponseDto,
  ExportProjectResponseDto,
  ValidateProjectResponseDto,
  ValidationFindingDto
} from '../client/types';
import { project } from './project.svelte';

/** UI locale id → strict language-folder form (the export writer joins it
 *  into output paths; the backend refuses any other form). */
export const FOLDER_FORM: Record<string, string> = {
  ru: 'Russian',
  de: 'German',
  en: 'English'
};

export function folderForm(localeId: string): string {
  return FOLDER_FORM[localeId] ?? localeId;
}

class ContractOpsStore {
  validating = $state(false);
  validateResult = $state<ValidateProjectResponseDto | null>(null);
  exporting = $state(false);
  exportResult = $state<ExportProjectResponseDto | null>(null);
  buildingMod = $state(false);
  buildModResult = $state<BuildModProjectResponseDto | null>(null);
  diagnosing = $state(false);
  diagnoseResult = $state<DiagnoseResponseDto | null>(null);
  /** Last typed contract failure (shown verbatim by the panels). */
  error = $state<string | null>(null);
  /** Caller-specified output directories (export + diagnose bundle). */
  exportOutDir = $state('');
  diagnoseOutDir = $state('');

  private cc() {
    return clientInstance.getClient();
  }

  private requireActive(): string {
    if (project.source !== 'contract' || !project.contractProjectId) {
      throw new ContractClientError('contract_violation', 'no live contract project is open');
    }
    return project.contractProjectId;
  }

  reset(): void {
    this.validating = false;
    this.validateResult = null;
    this.exporting = false;
    this.exportResult = null;
    this.buildingMod = false;
    this.buildModResult = null;
    this.diagnosing = false;
    this.diagnoseResult = null;
    this.error = null;
  }

  private fail(e: unknown): void {
    this.error =
      e instanceof ContractClientError ? `${e.code}: ${e.message}` : String(e);
  }

  /** Read-only validate over the trusted session state. On success the
   *  findings flow back into the workspace: an ERROR finding on a resolved
   *  structural id marks that entry's validation badge (warnings do not —
   *  026 semantics: warnings are reported, not failures). */
  async runValidate(): Promise<boolean> {
    let projectId: string;
    try {
      projectId = this.requireActive();
    } catch (e) {
      this.fail(e);
      return false;
    }
    this.validating = true;
    this.validateResult = null;
    this.error = null;
    try {
      const res = await this.cc().validateProject(
        projectId,
        project.contractEpoch,
        folderForm(project.targetLocale)
      );
      this.validateResult = res;
      this.applyFindingsToEntries(res.findings);
      return true;
    } catch (e) {
      this.fail(e);
      return false;
    } finally {
      this.validating = false;
    }
  }

  private applyFindingsToEntries(findings: ValidationFindingDto[]): void {
    for (const f of findings) {
      if (!f.id || f.severity !== 'error') continue;
      const kind = f.id.kind;
      const key = f.id.key;
      const entry = project.byId(
        f.id.def_type ? `${kind}:${key}:${f.id.def_type}` : `${kind}:${key}`
      );
      if (!entry) continue;
      entry.validation = 'issues';
      entry.validationIssues = [f.message];
    }
  }

  /** Export into the explicit caller-chosen out directory. */
  async runExport(outDir: string): Promise<boolean> {
    let projectId: string;
    try {
      projectId = this.requireActive();
    } catch (e) {
      this.fail(e);
      return false;
    }
    const dir = outDir.trim();
    if (!dir) {
      this.error = 'export: choose an output directory first';
      return false;
    }
    this.exporting = true;
    this.exportResult = null;
    this.error = null;
    try {
      this.exportResult = await this.cc().exportProject(
        projectId,
        project.contractEpoch,
        dir,
        folderForm(project.targetLocale)
      );
      return true;
    } catch (e) {
      this.fail(e);
      return false;
    } finally {
      this.exporting = false;
    }
  }

  /** FULL drop-in mod package (About `<ModMetaData>` + Languages) into the
   *  explicit caller-chosen out directory — same guard partition and DTO
   *  pattern as the export; the folder drops straight into the game's Mods
   *  directory without a terminal. */
  async runBuildMod(outDir: string): Promise<boolean> {
    let projectId: string;
    try {
      projectId = this.requireActive();
    } catch (e) {
      this.fail(e);
      return false;
    }
    const dir = outDir.trim();
    if (!dir) {
      this.error = 'build_mod: choose an output directory first';
      return false;
    }
    this.buildingMod = true;
    this.buildModResult = null;
    this.error = null;
    try {
      this.buildModResult = await this.cc().buildModProject(
        projectId,
        project.contractEpoch,
        dir,
        folderForm(project.targetLocale)
      );
      return true;
    } catch (e) {
      this.fail(e);
      return false;
    } finally {
      this.buildingMod = false;
    }
  }

  /** Sanitized bundle over the last failed operation. */
  async runDiagnose(outDir: string): Promise<boolean> {
    let projectId: string;
    try {
      projectId = this.requireActive();
    } catch (e) {
      this.fail(e);
      return false;
    }
    const dir = outDir.trim();
    if (!dir) {
      this.error = 'diagnose: choose a bundle output directory first';
      return false;
    }
    this.diagnosing = true;
    this.diagnoseResult = null;
    this.error = null;
    try {
      this.diagnoseResult = await this.cc().diagnoseProject(projectId, dir);
      return true;
    } catch (e) {
      this.fail(e);
      return false;
    } finally {
      this.diagnosing = false;
    }
  }
}

export const contractops = new ContractOpsStore();
