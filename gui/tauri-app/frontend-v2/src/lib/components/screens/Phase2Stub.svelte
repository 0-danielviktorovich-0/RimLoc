<script lang="ts">
  // Phase-2 route stubs (mandate §6/§15/§16): Settings, AI Provider Manager
  // and Help render their planned section lists so the routes are real, while
  // the full screens are owned by phase-2 workers.
  import Icon from '../Icon.svelte';
  import { t } from '../../../i18n/store.svelte';
  import { router, type RouteId } from '../../router.svelte';

  let { route }: { route: Extract<RouteId, 'settings' | 'providers' | 'help'> } = $props();

  const CONFIG: Record<
    'settings' | 'providers' | 'help',
    { title: string; desc: string; icon: string; items: string[]; back?: RouteId }
  > = {
    settings: {
      title: 'settings.title',
      desc: 'settings.desc',
      icon: 'settings',
      items: [
        'settings.section.general',
        'settings.section.rimworld',
        'settings.section.translation',
        'settings.section.ai',
        'settings.section.editor',
        'settings.section.appearance',
        'settings.section.advanced'
      ]
    },
    providers: {
      title: 'providers.title',
      desc: 'providers.desc',
      icon: 'cpu',
      items: [
        'providers.example.zai',
        'providers.example.anthropic',
        'providers.example.ollama',
        'providers.example.custom'
      ],
      back: 'settings'
    },
    help: {
      title: 'help.title',
      desc: 'help.desc',
      icon: 'help',
      items: [
        'help.item.quickstart',
        'help.item.replay',
        'help.item.shortcuts',
        'help.item.docs',
        'help.item.troubleshoot',
        'help.item.diagnose',
        'help.item.bugreport'
      ]
    }
  };

  const meta = $derived(CONFIG[route]);
</script>

<section class="stub" aria-labelledby="stub-heading" data-testid={`stub.${route}`}>
  <h1 id="stub-heading" class="title">
    <Icon name={meta.icon} size={22} />
    {t(meta.title)}
  </h1>
  <p class="desc">{t(meta.desc)}</p>

  <ul class="items">
    {#each meta.items as item (item)}
      <li class="item">
        <Icon name="chevron-right" size={14} />
        {t(item)}
      </li>
    {/each}
  </ul>

  <div class="actions">
    {#if meta.back}
      <button type="button" class="btn" data-testid="stub.back" onclick={() => router.navigate(meta.back!)}>
        <Icon name="arrow-left" size={14} />
        {t('common.back')}
      </button>
    {/if}
    <button type="button" class="btn" data-testid="stub.home" onclick={() => router.navigate('home')}>
      <Icon name="home" size={14} />
      {t('nav.home')}
    </button>
  </div>

  <p class="note">{t('phase2.note')}</p>
</section>

<style>
  .stub {
    width: min(640px, 100%);
    margin: 0 auto;
    padding: var(--space-8) var(--space-4);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .title {
    margin: 0;
    font-family: var(--font-heading);
    font-size: var(--text-heading-size);
    font-weight: var(--text-heading-weight);
    letter-spacing: var(--heading-tracking);
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }

  .desc {
    margin: 0;
    color: var(--color-muted-fg);
  }

  .items {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    display: flex;
    flex-direction: column;
  }

  .item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    border-bottom: 1px solid var(--color-border);
    font-size: var(--text-base-size);
  }

  .item:last-child {
    border-bottom: none;
  }

  .item :global(svg) {
    color: var(--color-muted-fg);
    flex: none;
  }

  .actions {
    display: flex;
    gap: var(--space-2);
  }

  .note {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }
</style>
