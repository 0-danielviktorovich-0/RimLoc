<script lang="ts">
  // App shell: global header + the current route + dev panel.
  // Routing is the dependency-free hash router (mandate phase 1); each route
  // maps to exactly one screen so parallel workers own their files.
  import AppHeader from './lib/components/AppHeader.svelte';
  import Home from './lib/components/screens/Home.svelte';
  import Wizard from './lib/components/screens/Wizard.svelte';
  import Workspace from './lib/components/screens/Workspace.svelte';
  import BuildStub from './lib/components/screens/BuildStub.svelte';
  import Existing from './lib/components/screens/Existing.svelte';
  import ChatBatchManager from './lib/components/chat/ChatBatchManager.svelte';
  import GlossaryEditor from './lib/components/screens/GlossaryEditor.svelte';
  import TMEditor from './lib/components/screens/TMEditor.svelte';
  import Phase2Stub from './lib/components/screens/Phase2Stub.svelte';
  import Diagnostics from './lib/components/screens/Diagnostics.svelte';
  import OnboardingCoach from './lib/components/OnboardingCoach.svelte';
  import ScenarioBrowser from './lib/components/ScenarioBrowser.svelte';
  import DevPanel from './lib/components/DevPanel.svelte';
  import StyleLab from './lib/components/StyleLab.svelte';
  import { router } from './lib/router.svelte';
  import { ui } from './lib/stores/ui.svelte';
  import { stylelab } from './lib/stores/stylelab.svelte';
  import { devMode } from './lib/stores/devmode.svelte';
  import { clientInstance } from './lib/client/instance.svelte';
  import { capability } from './lib/client/capability.svelte';
  import { t } from './i18n/store.svelte';

  // W-built boot gate: no Tauri bridge and no explicit dev/demo opt-in means
  // the honest configuration error — the mock is never a silent default.
  const bootMode = clientInstance.resolveMode();

  // Audit P1-5: fetch the capability report once at boot so the validate/
  // build/diagnostics CTAs can degrade honestly (disabled + reason) against
  // the REAL backend slice boundary instead of dead-ending in mock stubs.
  $effect(() => {
    if (bootMode !== 'none') void capability.ensure();
  });
</script>

{#if bootMode === 'none'}
  <main class="app-main">
    <section class="cfg-error" role="alert" aria-labelledby="cfg-title" data-testid="boot.config-error">
      <h1 id="cfg-title">{t('boot.configError.title')}</h1>
      <p class="cfg-text">{clientInstance.configError}</p>
      <p class="cfg-text">{t('boot.configError.hint')}</p>
    </section>
  </main>
{:else}
<AppHeader />

<main class="app-main">
  {#if router.route === 'home'}
    <Home state={ui.homeState} />
  {:else if router.route === 'wizard'}
    <Wizard />
  {:else if router.route === 'workspace' || router.route === 'review'}
    <!-- Deep links open Workspace with the matching tab (#/review from the wizard result). -->
    <Workspace initialTab={router.route === 'review' ? 'review' : 'editor'} />
  {:else if router.route === 'build'}
    <BuildStub />
  {:else if router.route === 'existing'}
    <Existing />
  {:else if router.route === 'chat'}
    <!-- Chat-batch manager (spec §10): also opened from the wizard result. -->
    <ChatBatchManager />
  {:else if router.route === 'glossary'}
    <!-- Project glossary editor (mandate §10, W4). -->
    <GlossaryEditor />
  {:else if router.route === 'tm'}
    <!-- Translation memory editor (mandate §11, W4). -->
    <TMEditor />
  {:else if router.route === 'diagnostics'}
    <!-- Diagnostics workstation: controlled failure → causal context →
         sanitized bundle (mandate §16-§17, W5). -->
    <Diagnostics />
  {:else}
    <Phase2Stub route={router.route} />
  {/if}
</main>

<!-- W6: developer tooling (dev panel, scenario browser) is gated behind ONE
     explicit dev/testing mode — vite dev, ?dev=1 or a persisted opt-in. The
     global Demo-data badge above stays unconditional in every build. -->
{#if devMode.enabled}
  <DevPanel />
  <ScenarioBrowser />
{/if}

<!-- W6: anchored product tour lives at shell level — the guided demo script
     spans Home → Workspace → Review → Build, so it survives route changes.
     This is product onboarding, NOT dev tooling: never gated. -->
<OnboardingCoach />

<!-- Style Lab is dev-only: gated behind ?stylelab=1 / localStorage opt-in. -->
{#if stylelab.enabled}
  <StyleLab />
{/if}

<style>
  .app-main {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .cfg-error {
    margin: var(--space-8) auto;
    max-width: 560px;
    border: 1px solid var(--color-destructive);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    padding: var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .cfg-error h1 {
    margin: 0;
    font-family: var(--font-heading);
    font-size: var(--text-heading-size);
    color: var(--color-destructive);
  }

  .cfg-text {
    margin: 0;
    color: var(--color-muted-fg);
  }
</style>
{/if}
