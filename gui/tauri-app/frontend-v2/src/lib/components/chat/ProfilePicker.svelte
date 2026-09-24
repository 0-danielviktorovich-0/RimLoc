<script lang="ts">
  // Chat profiles: behavior defaults per target chat (how it likes prompts
  // and replies). These are NOT secrets and never hold keys — just defaults.
  // The profile recommended for the current target locale gets a badge.
  import { t } from '../../../i18n/store.svelte';
  import { chat, CHAT_PROFILES, type ChatProfileId } from '../../stores/chatbatch.svelte';

  // Labels resolve through t() at render time, so they follow the UI locale.
  const profileLabel = (id: ChatProfileId): string => t(`chat.profile.${id}`);
  const profileDesc = (id: ChatProfileId): string => t(`chat.profile.${id}Desc`);
  const isRecommended = (id: ChatProfileId): boolean => chat.recommendedProfile === id;
</script>

<section class="panel" aria-labelledby="profile-heading">
  <h2 id="profile-heading">{t('chat.profile.title')}</h2>
  <p class="desc">{t('chat.profile.desc')}</p>

  <div class="grid" role="radiogroup" aria-label={t('chat.profile.title')}>
    {#each CHAT_PROFILES as id (id)}
      <button
        type="button"
        class="profile"
        role="radio"
        aria-checked={chat.profile === id}
        data-testid={`chat.profile.${id}`}
        onclick={() => chat.setProfile(id)}
      >
        <span class="name">
          {profileLabel(id)}
          {#if isRecommended(id)}
            <span class="rec">{t('chat.summary.recommended')}</span>
          {/if}
        </span>
        <span class="pdesc">{profileDesc(id)}</span>
      </button>
    {/each}
  </div>
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
  }

  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
  }

  .desc {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-2);
  }

  .profile {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    text-align: left;
    transition:
      border-color var(--motion-fast) var(--ease-out),
      background var(--motion-fast) var(--ease-out);
  }

  .profile:hover {
    border-color: var(--color-border-strong);
    background: var(--color-muted);
  }

  .profile[aria-checked='true'] {
    border-color: var(--color-primary);
    background: var(--nav-active-bg);
  }

  .name {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-weight: 600;
  }

  .rec {
    padding: 0 var(--space-1);
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-sm);
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    font-weight: 400;
  }

  .pdesc {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  @media (max-width: 560px) {
    .grid {
      grid-template-columns: 1fr;
    }
  }
</style>
