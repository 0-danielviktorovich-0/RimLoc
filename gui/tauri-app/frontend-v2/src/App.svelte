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
  import Phase2Stub from './lib/components/screens/Phase2Stub.svelte';
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
  {:else}
    <Phase2Stub route={router.route} />
  {/if}
</main>

<DevPanel />

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
