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
</script>

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

<DevPanel />

<!-- W6: anchored product tour lives at shell level — the guided demo script
     spans Home → Workspace → Review → Build, so it survives route changes. -->
<OnboardingCoach />

<!-- W6: dev scenario browser + deep-link runner (?scenario=<id>); the picker
     opens from the dev panel, deep-links apply regardless. -->
<ScenarioBrowser />

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
</style>
