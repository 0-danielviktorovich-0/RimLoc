<script lang="ts">
  // App shell: global header + the current screen + dev panel.
  // Routing is a plain rune (no router needed for two screens).
  import AppHeader from './lib/components/AppHeader.svelte';
  import Home from './lib/components/screens/Home.svelte';
  import Workspace from './lib/components/screens/Workspace.svelte';
  import DevPanel from './lib/components/DevPanel.svelte';
  import StyleLab from './lib/components/StyleLab.svelte';
  import { ui } from './lib/stores/ui.svelte';
  import { stylelab } from './lib/stores/stylelab.svelte';
</script>

<AppHeader />

<main class="app-main">
  {#if ui.screen === 'home'}
    <Home state={ui.homeState} />
  {:else}
    <Workspace />
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
