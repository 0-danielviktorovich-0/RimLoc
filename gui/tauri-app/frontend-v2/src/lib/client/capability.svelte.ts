// Capability store (audit P1-5): the handshake CapabilityReport is the
// honest-degradation seam (UI_SDK mandate: "Capabilities отражают реальные
// операции; typed unsupported допустим в промежуточном срезе"). Before this
// store the report was thrown away after the version check, so the UI had no
// way to know that validate/build/diagnostics are NOT wired yet — it showed
// enabled CTAs whose flows dead-end in mock stubs.
//
// One-shot at boot (App.svelte): ensure() runs the handshake once per client
// instance and caches the report; failures are kept as a typed error string
// and the store simply stays "unknown" — the UI keeps its current behavior
// and the flow fails honestly at call time, never silently pretends support.
//
// The store is deliberately PURE: it never imports the project store. Gating
// a CTA combines two independent truths at the component:
//   disabled={project.source === 'contract' && capability.state(CAP) === false}
// — demo/fixture projects keep their marked demo flows; only a REAL contract
// project is degraded by an unsupported capability.
import type { CapabilityReportDto } from './types';
import { ContractClientError } from './client';
import { clientInstance } from './instance.svelte';

/** Capability names as the Rust side reports them (contract.rs
 * capability_report). Keep in sync with crates/rimloc-services. */
export const CAP_VALIDATE = 'validate_via_contract';
export const CAP_BUILD = 'build_export';
export const CAP_SOURCE_ACTIONS = 'source_inspector_actions';
export const CAP_DIAGNOSTICS = 'diagnostics_bundle';
export const CAP_PROVIDERS = 'providers_settings';

class CapabilityStore {
  /** The last handshake report; null until ensure() resolves (or mode 'none'). */
  report = $state<CapabilityReportDto | null>(null);
  /** Typed handshake failure ('contract_violation: …'), null on success. */
  error = $state<string | null>(null);
  private pending: Promise<void> | null = null;

  /** Run the handshake once and cache the capability report. Idempotent. */
  ensure(): Promise<void> {
    if (this.report) return Promise.resolve();
    if (!this.pending) {
      this.pending = (async () => {
        try {
          const hs = await clientInstance.getClient().handshake();
          this.report = hs.capabilities;
          this.error = null;
        } catch (e) {
          // Handshake failure is honest, non-fatal state: the UI stays
          // "unknown" (CTAs keep their current behavior) instead of
          // pretending anything about support.
          this.error =
            e instanceof ContractClientError ? `${e.code}: ${e.message}` : String(e);
        }
      })();
    }
    return this.pending;
  }

  /** true/false once the report is in; null = not known yet. Strictly the
   * `supported` list — a capability named in NEITHER list is unsupported. */
  state(cap: string): boolean | null {
    if (!this.report) return null;
    return this.report.supported.includes(cap);
  }

  /** Backend's reason string for an unsupported capability, if reported. */
  reason(cap: string): string | null {
    return this.report?.unsupported.find((u) => u.capability === cap)?.reason ?? null;
  }

  /** Test/dev reset (mirrors the other stores' reset seams). */
  reset(): void {
    this.report = null;
    this.error = null;
    this.pending = null;
  }
}

export const capability = new CapabilityStore();
