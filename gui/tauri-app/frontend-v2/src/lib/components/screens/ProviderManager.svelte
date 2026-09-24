<script lang="ts">
  // AI Provider Manager (mandate §6-7: provider templates + instances, spec
  // §12). Two distinct sections:
  //
  //   TEMPLATES — protocol families (Z.AI / OpenAI-compatible / Anthropic /
  //   Ollama / Custom). Clicking a template opens the add form prefilled.
  //   INSTANCES — concrete configured providers ("Z.AI Personal", "Local
  //   Ollama", "My VPS") with the full lifecycle: enable/disable, default
  //   mark, rename, duplicate, edit, remove, connection test.
  //
  // Credential semantics (W4.5 requirement #1, behavior-review 008):
  //   - Duplicate copies ONLY non-secret configuration (name / base URL /
  //     model / protocol). The secret itself never travels; the copy gets an
  //     explicit credential decision: reuse a shared keychain reference
  //     (possible only when the source HAS a resolvable reference), or start
  //     in "needs credential" state. Local no-key families duplicate plainly
  //     — honestly offline, no paid-credential claims.
  //   - Credential identity is an OPAQUE mock keychain handle (`credRef`),
  //     never a secret and never exported. Own and shared providers resolve
  //     the same handle; replacing the key on a shared copy detaches it into
  //     its own handle. The keychain entry itself is never removed by the UI
  //     (see removeWarn), so a surviving shared copy keeps resolving honestly.
  //     Invariant kept after every mutation: hasKey=true ⇒ credRef present —
  //     a provider can never claim "connected" on a non-resolvable credential.
  //   - Export serializes the provider WITHOUT any credential fields at all —
  //     auth / key flags / credRef are absent keys, never empty strings.
  //   - Export reports success only after the clipboard write fulfilled; on
  //     rejection or a missing clipboard API it falls back to a selectable
  //     JSON block the user can copy manually.
  //
  // Secrets are NEVER rendered: the key row shows a keychain flag only
  // ("•••• (keychain)"), never a value. Everything here is mock state — no
  // provider is contacted. Instance changes are synced back to the legacy
  // providers store so the Settings → AI summary stays truthful.
  import Icon from '../Icon.svelte';
  import { t } from '../../../i18n/store.svelte';
  import { router } from '../../router.svelte';
  import { providers, type ProviderId, type ProviderStatus } from '../../stores/providers.svelte';

  type Family = ProviderId;
  type Discovery = 'auto' | 'manual';
  type Auth = 'keychain' | 'none';
  /**
   * Provenance of the credential a provider instance resolves to:
   *   own          — this instance has its own keychain entry;
   *   shared-ref   — resolves to the SAME keychain entry as the original
   *                  (reference reused at duplicate, never copied);
   *   missing      — credential required by the protocol but not present;
   *   not_required — protocol family needs no key (local services).
   */
  type CredentialState = 'own' | 'shared-ref' | 'missing' | 'not_required';

  interface Instance {
    id: string;
    name: string;
    family: Family;
    baseUrl: string;
    model: string;
    discovery: Discovery;
    auth: Auth;
    /** Keychain flag only — no secret ever lives in this component. */
    hasKey: boolean;
    /**
     * Opaque mock keychain handle this instance resolves to (e.g. "kc-1").
     * An identity for sharing — NOT a secret, never rendered, never exported.
     * Null = no resolvable credential.
     */
    credRef: string | null;
    /** Where the credential reference comes from (see CredentialState). */
    credential: CredentialState;
    enabled: boolean;
    isDefault: boolean;
    status: ProviderStatus;
  }

  interface FamilyMeta {
    id: Family;
    nameKey: string;
    descKey: string;
    privacy: 'cloud' | 'local';
    defaultBaseUrl: string;
    defaultModels: string[];
  }

  const FAMILIES: FamilyMeta[] = [
    {
      id: 'zai',
      nameKey: 'providers.example.zai',
      descKey: 'provider.zai.desc',
      privacy: 'cloud',
      defaultBaseUrl: 'https://api.z.ai/api/anthropic',
      defaultModels: ['glm-4.6', 'glm-4.5-air']
    },
    {
      id: 'openai',
      nameKey: 'providers.example.openai',
      descKey: 'provider.openai.desc',
      privacy: 'cloud',
      defaultBaseUrl: 'https://api.openai.com/v1',
      defaultModels: ['gpt-4o', 'gpt-4o-mini']
    },
    {
      id: 'anthropic',
      nameKey: 'providers.example.anthropic',
      descKey: 'provider.anthropic.desc',
      privacy: 'cloud',
      defaultBaseUrl: 'https://api.anthropic.com',
      defaultModels: ['claude-sonnet-4', 'claude-haiku-4']
    },
    {
      id: 'ollama',
      nameKey: 'providers.example.ollama',
      descKey: 'provider.ollama.desc',
      privacy: 'local',
      defaultBaseUrl: 'http://localhost:11434',
      defaultModels: ['llama3.1:8b', 'qwen2.5:7b']
    },
    {
      id: 'custom',
      nameKey: 'providers.example.custom',
      descKey: 'provider.custom.desc',
      privacy: 'cloud',
      defaultBaseUrl: 'https://your-endpoint.example/v1',
      defaultModels: ['custom-model']
    }
  ];

  function familyMeta(id: Family): FamilyMeta {
    const m = FAMILIES.find((f) => f.id === id);
    if (!m) throw new Error(`unknown family: ${id}`);
    return m;
  }

  let nextNum = 4;
  /** Mock keychain handle sequence — identities only, never secret values. */
  let nextKc = 2;

  /** A fresh OWN keychain handle for an instance that saves a key. */
  function newKcRef(): string {
    return `kc-${nextKc++}`;
  }

  // Mock configured instances: one healthy cloud default, one local offline,
  // one custom not-yet-tested endpoint — three different lifecycle states.
  // `credential`/`credRef` record provenance and identity; never secret values.
  let instances = $state<Instance[]>([
    {
      id: 'inst-1',
      name: 'Z.AI Personal',
      family: 'zai',
      baseUrl: 'https://api.z.ai/api/anthropic',
      model: 'glm-4.6',
      discovery: 'auto',
      auth: 'keychain',
      hasKey: true,
      credRef: 'kc-1',
      credential: 'own',
      enabled: true,
      isDefault: true,
      status: 'connected'
    },
    {
      id: 'inst-2',
      name: 'Local Ollama',
      family: 'ollama',
      baseUrl: 'http://localhost:11434',
      model: 'llama3.1:8b',
      discovery: 'auto',
      auth: 'none',
      hasKey: false,
      credRef: null,
      credential: 'not_required',
      enabled: true,
      isDefault: false,
      status: 'offline'
    },
    {
      id: 'inst-3',
      name: 'My VPS',
      family: 'custom',
      baseUrl: 'https://my-vps.example/v1',
      model: 'custom-model',
      discovery: 'manual',
      auth: 'keychain',
      hasKey: false,
      credRef: null,
      credential: 'missing',
      enabled: false,
      isDefault: false,
      status: 'not_configured'
    }
  ]);

  // ---- add/edit form ------------------------------------------------------
  interface ProviderForm {
    editingId: string | null;
    name: string;
    family: Family;
    baseUrl: string;
    auth: Auth;
    discovery: Discovery;
    model: string;
    hasKey: boolean;
  }

  let form = $state<ProviderForm | null>(null);
  let flash = $state<Record<string, string | undefined>>({});
  let flashTimer: ReturnType<typeof setTimeout> | undefined;
  /** Two-step remove: first click arms, second click deletes. */
  let confirmingRemove = $state<string | null>(null);
  /** Inline rename: id of the instance whose name is an input right now. */
  let renaming = $state<string | null>(null);
  let renameDraft = $state('');
  let timers: ReturnType<typeof setTimeout>[] = [];

  $effect(() => {
    return () => {
      clearTimeout(flashTimer);
      timers.forEach(clearTimeout);
    };
  });

  function showFlash(id: string, key: string) {
    flash = { ...flash, [id]: key };
    clearTimeout(flashTimer);
    flashTimer = setTimeout(() => {
      flash = { ...flash, [id]: undefined };
    }, 2200);
  }

  function openAddForm(family?: Family) {
    const f = family ? familyMeta(family) : FAMILIES[0];
    form = {
      editingId: null,
      name: '',
      family: f.id,
      baseUrl: f.defaultBaseUrl,
      auth: f.privacy === 'local' ? 'none' : 'keychain',
      discovery: 'auto',
      model: f.defaultModels[0],
      hasKey: false
    };
  }

  function openEditForm(inst: Instance) {
    form = {
      editingId: inst.id,
      name: inst.name,
      family: inst.family,
      baseUrl: inst.baseUrl,
      auth: inst.auth,
      discovery: inst.discovery,
      model: inst.model,
      hasKey: inst.hasKey
    };
  }

  function pickFamilyInForm(family: Family) {
    if (!form || form.editingId !== null) return; // family is fixed while editing
    const f = familyMeta(family);
    form.family = family;
    form.baseUrl = f.defaultBaseUrl;
    form.model = f.defaultModels[0];
    form.auth = f.privacy === 'local' ? 'none' : 'keychain';
  }

  /** Credential provenance implied by the form state at creation/edit. */
  function credentialFor(auth: Auth, hasKey: boolean): CredentialState {
    if (auth === 'none') return 'not_required';
    return hasKey ? 'own' : 'missing';
  }

  /**
   * Honest-state invariant (behavior-review 008): a provider that claims a
   * key MUST resolve a keychain handle; otherwise it downgrades to a truthful
   * "needs credential". Called after every credential-affecting mutation.
   */
  function revalidateCredentials() {
    for (const inst of instances) {
      if (inst.hasKey && !inst.credRef) {
        inst.hasKey = false;
        inst.credential = inst.auth === 'none' ? 'not_required' : 'missing';
        if (inst.status === 'connected') inst.status = 'not_configured';
      }
      if (!inst.hasKey && inst.credRef) inst.credRef = null;
    }
  }

  function saveForm() {
    if (!form) return;
    const name = form.name.trim() || t(`providers.example.${form.family}`);
    if (form.editingId) {
      const inst = instances.find((i) => i.id === form!.editingId);
      if (inst) {
        inst.name = name;
        inst.baseUrl = form.baseUrl.trim() || familyMeta(inst.family).defaultBaseUrl;
        inst.auth = form.auth;
        inst.discovery = form.discovery;
        inst.model = form.model.trim() || inst.model;
        if (form.hasKey) {
          inst.hasKey = true;
          // Editing keeps an existing own handle; a fresh key on a keyless
          // instance mints its own handle (never manufactures a shared one).
          if (!inst.credRef) inst.credRef = newKcRef();
        }
        // Editing may move auth/hasKey — recompute credential provenance
        // unless the instance intentionally shares another entry's reference.
        if (inst.credential !== 'shared-ref') inst.credential = credentialFor(inst.auth, inst.hasKey);
        if (inst.auth === 'none') inst.credential = 'not_required';
      }
      showFlash(form.editingId, 'providers.action.saved');
    } else {
      const id = `inst-${nextNum++}`;
      instances = [
        ...instances,
        {
          id,
          name,
          family: form.family,
          baseUrl: form.baseUrl.trim() || familyMeta(form.family).defaultBaseUrl,
          model: form.model.trim() || familyMeta(form.family).defaultModels[0],
          discovery: form.discovery,
          auth: form.auth,
          hasKey: form.hasKey,
          credRef: null,
          credential: credentialFor(form.auth, form.hasKey),
          enabled: true,
          isDefault: false,
          status: form.family === 'ollama' ? 'offline' : 'not_configured'
        }
      ];
      form = null;
      revalidateCredentials();
      syncLegacy();
      return;
    }
    form = null;
    revalidateCredentials();
    syncLegacy();
  }

  function cancelForm() {
    form = null;
  }

  /** Mock keychain prompt: flips the flag, no secret enters the UI.
   *  Saving a key here gives the instance its OWN keychain entry — a shared
   *  reference is only produced by the explicit duplicate decision, and
   *  saving over a shared reference DETACHES the copy into its own entry
   *  (behavior-review 008 #2). */
  function replaceKey(id: string) {
    const inst = instances.find((i) => i.id === id);
    if (inst) {
      inst.hasKey = true;
      if (inst.auth === 'keychain') {
        if (inst.credential === 'shared-ref' || !inst.credRef) {
          // Detach: the copy now owns a fresh keychain entry.
          inst.credRef = newKcRef();
          inst.credential = 'own';
        }
      }
    }
    showFlash(id, 'providers.test.keySaved');
    revalidateCredentials();
    syncLegacy();
  }

  function testConnection(id: string) {
    const inst = instances.find((i) => i.id === id);
    if (!inst) return;
    inst.status = 'testing';
    const timer = setTimeout(() => {
      const target = instances.find((i) => i.id === id);
      if (!target) return;
      if (!target.enabled) target.status = 'not_configured';
      else if (familyMeta(target.family).privacy === 'local') target.status = 'offline';
      else target.status = target.hasKey ? 'connected' : 'not_configured';
      syncLegacy();
    }, 800);
    timers.push(timer);
  }

  function toggleEnabled(id: string) {
    const inst = instances.find((i) => i.id === id);
    if (!inst) return;
    inst.enabled = !inst.enabled;
    if (!inst.enabled) {
      inst.isDefault = false;
      inst.status = 'not_configured';
    }
    // Keep exactly one default among enabled instances.
    if (inst.enabled && !instances.some((i) => i.enabled && i.isDefault)) {
      inst.isDefault = true;
    }
    syncLegacy();
  }

  function makeDefault(id: string) {
    for (const inst of instances) inst.isDefault = inst.id === id;
    syncLegacy();
  }

  function startRename(inst: Instance) {
    renaming = inst.id;
    renameDraft = inst.name;
  }

  function commitRename(id: string) {
    const inst = instances.find((i) => i.id === id);
    if (inst && renameDraft.trim()) inst.name = renameDraft.trim();
    renaming = null;
  }

  /** Two-step duplicate: first click opens the credential decision, the
   *  chosen option completes it (W4.5 requirement #1 — duplicate copies ONLY
   *  non-secret configuration and decides the credential explicitly). */
  let duplicating = $state<string | null>(null);

  function startDuplicate(id: string) {
    duplicating = duplicating === id ? null : id;
  }

  function duplicate(id: string, mode: 'shared-ref' | 'missing') {
    const src = instances.find((i) => i.id === id);
    if (!src) return;
    // Handler-side guard (behavior-review 008 #1): a shared reference can
    // only be reused when the source actually resolves one.
    if (mode === 'shared-ref' && !(src.auth === 'keychain' && src.hasKey && src.credRef)) return;
    duplicating = null;
    // Spread of the non-secret projection, not of the source object: the
    // keychain flag is never carried over as a cloneable value.
    const copy: Instance = {
      ...nonSecretConfig(src),
      id: `inst-${nextNum++}`,
      name: t('providers.inst.copyOf', { name: src.name }),
      auth: src.auth,
      // Reuse RESOLVES THE SAME keychain identity; it is a reference, not a
      // copied value. Needs-credential starts with nothing resolvable.
      hasKey: mode === 'shared-ref',
      credRef: mode === 'shared-ref' ? src.credRef : null,
      credential: src.auth === 'none' ? 'not_required' : mode,
      enabled: true,
      isDefault: false,
      // Same keychain identity behaves like the source; a keyless copy starts
      // unconfigured; a local no-key family stays honestly offline.
      status:
        mode === 'shared-ref' || src.auth === 'none' ? src.status : 'not_configured'
    };
    instances = [...instances, copy];
    revalidateCredentials();
    syncLegacy();
    showFlash(
      copy.id,
      mode === 'shared-ref'
        ? 'providers.duplicate.sharedFlash'
        : src.auth === 'none'
          ? 'providers.duplicate.localFlash'
          : 'providers.duplicate.missingFlash'
    );
  }

  /**
   * Non-secret projection of an instance: name / base URL / model / protocol
   * (family + discovery). No auth, no key flags, no credential provenance,
   * no keychain handle — the same shape `exportInstance` writes, so what you
   * export is what a duplicate would carry. Reference IDs are excluded.
   */
  function nonSecretConfig(src: Instance) {
    return {
      name: src.name,
      family: src.family,
      baseUrl: src.baseUrl,
      model: src.model,
      discovery: src.discovery
    };
  }

  /** Clipboard outcome for the export flow (behavior-review 008 #3). */
  let exportFallback: { id: string; json: string } | null = $state(null);

  /**
   * Mock export: serializes the NON-SECRET provider configuration as JSON.
   * Credential fields (auth / hasKey / credential / credRef) are omitted
   * entirely — absent keys, never empty strings. Success is reported only
   * after the clipboard write FULFILLED; on rejection or a missing clipboard
   * API the JSON is shown as a selectable block to copy manually instead of
   * a false "done".
   */
  async function exportInstance(id: string) {
    const src = instances.find((i) => i.id === id);
    if (!src) return;
    const json = JSON.stringify(nonSecretConfig(src), null, 2);
    const clipboard = navigator.clipboard;
    if (!clipboard?.writeText) {
      // Clipboard API unavailable — reviewable fallback, no false success.
      exportFallback = { id, json };
      showFlash(id, 'providers.export.unavailable');
      return;
    }
    try {
      await clipboard.writeText(json);
      exportFallback = null;
      showFlash(id, 'providers.export.done');
    } catch {
      // Write denied/failed — selectable JSON fallback, explicit message.
      exportFallback = { id, json };
      showFlash(id, 'providers.export.denied');
    }
  }

  function removeInstance(id: string) {
    if (confirmingRemove !== id) {
      confirmingRemove = id;
      return;
    }
    confirmingRemove = null;
    instances = instances.filter((i) => i.id !== id);
    // Hand the default role to a surviving enabled instance.
    if (!instances.some((i) => i.enabled && i.isDefault)) {
      const heir = instances.find((i) => i.enabled);
      if (heir) heir.isDefault = true;
    }
    // The keychain entry itself is never removed (removeWarn says so), so
    // shared copies of a removed provider keep resolving the same handle —
    // no silent "connected" lie. revalidate enforces hasKey ⇒ credRef.
    revalidateCredentials();
    syncLegacy();
  }

  /** Sync aggregate state back into the legacy providers store so the
   *  Settings → AI summary reflects the instance list (mock-level coherence). */
  function syncLegacy() {
    for (const f of FAMILIES) {
      const inst = instances.find((i) => i.family === f.id && i.enabled);
      if (inst) {
        providers.setStatus(f.id, inst.status);
        providers.setBaseUrl(f.id, inst.baseUrl);
        providers.setModel(f.id, inst.model);
        providers.setHasKey(f.id, inst.hasKey);
      } else {
        providers.setStatus(f.id, 'not_configured');
        providers.setHasKey(f.id, false);
      }
    }
  }

  const STATUS_ICON: Record<ProviderStatus, string> = {
    connected: 'circle-check',
    not_configured: 'info',
    offline: 'warning',
    testing: 'clock'
  };
</script>

<section class="providers" aria-labelledby="providers-heading">
  <div class="head">
    <h1 id="providers-heading" class="title">
      <Icon name="cpu" size={20} />
      {t('providers.title')}
    </h1>
    <p class="subtitle">{t('providers.subtitle')}</p>
  </div>

  <button type="button" class="btn back" data-testid="providers.back" onclick={() => router.navigate('settings')}>
    <Icon name="arrow-left" size={14} />
    {t('nav.settings')}
  </button>

  <!-- ==================== configured instances ==================== -->
  <h2 class="section-title" data-testid="providers.instances.title">{t('providers.instances.title')}</h2>

  {#if instances.length === 0}
    <p class="empty" data-testid="providers.instances.empty">{t('providers.instances.empty')}</p>
  {:else}
    <div class="cards">
      {#each instances as inst (inst.id)}
        {@const meta = familyMeta(inst.family)}
        <article
          class="card"
          class:disabled={!inst.enabled}
          data-testid={`providers.inst.${inst.id}`}
          data-cred-ref={inst.credRef ?? ''}
        >
          <header class="card-head">
            <span class="card-icon"><Icon name="cpu" size={18} /></span>
            <div class="card-title">
              {#if renaming === inst.id}
                <span class="rename-row">
                  <input
                    type="text"
                    bind:value={renameDraft}
                    aria-label={t('providers.inst.rename')}
                    data-testid={`providers.inst.renameInput.${inst.id}`}
                    onkeydown={(e) => {
                      if (e.key === 'Enter') commitRename(inst.id);
                      if (e.key === 'Escape') renaming = null;
                    }}
                  />
                  <button type="button" class="btn subtle" data-testid={`providers.inst.renameSave.${inst.id}`} onclick={() => commitRename(inst.id)}>
                    <Icon name="check" size={13} />
                  </button>
                </span>
              {:else}
                <span class="name-row">
                  <span class="name">{inst.name}</span>
                  {#if inst.isDefault}
                    <span class="badge default" data-testid={`providers.inst.default.${inst.id}`}>
                      <Icon name="check" size={11} />
                      {t('providers.inst.default')}
                    </span>
                  {/if}
                </span>
                <span class="desc">{t(meta.nameKey)}</span>
              {/if}
            </div>
            <label class="switch" title={t(inst.enabled ? 'providers.inst.enabled' : 'providers.inst.disabled')}>
              <input
                type="checkbox"
                checked={inst.enabled}
                data-testid={`providers.inst.toggle.${inst.id}`}
                onchange={() => toggleEnabled(inst.id)}
              />
              <span class="switch-label">{t(inst.enabled ? 'providers.inst.enabled' : 'providers.inst.disabled')}</span>
            </label>
            <span class={`status st-${inst.status}`} data-testid={`providers.inst.status.${inst.id}`}>
              <Icon name={STATUS_ICON[inst.status]} size={14} />
              {t(`providers.status.${inst.status}`)}
            </span>
          </header>

          <dl class="meta">
            <div class="meta-row">
              <dt>{t('providers.inst.model')}</dt>
              <dd class="mono">{inst.model}{inst.discovery === 'auto' ? ` · ${t('providers.form.discovery.auto')}` : ''}</dd>
            </div>
            <div class="meta-row">
              <dt>{t('providers.baseUrl')}</dt>
              <dd class="mono">{inst.baseUrl}</dd>
            </div>
            <div class="meta-row">
              <dt>{t('providers.form.key')}</dt>
              <dd class="key-cell">
                <span class="mono" data-testid={`providers.inst.key.${inst.id}`}>
                  {inst.hasKey
                    ? inst.credential === 'shared-ref'
                      ? `•••• •••• (${t('providers.inst.cred.shared')})`
                      : '•••• •••• (keychain)'
                    : t('providers.key.none')}
                </span>
                {#if inst.credential === 'shared-ref'}
                  <span class="badge cred-shared" data-testid={`providers.inst.credShared.${inst.id}`} title={t('providers.inst.cred.sharedHint')}>
                    <Icon name="link" size={11} />
                    {t('providers.inst.cred.shared')}
                  </span>
                {:else if inst.credential === 'missing'}
                  <span class="badge cred-missing" data-testid={`providers.inst.credMissing.${inst.id}`}>
                    <Icon name="warning" size={11} />
                    {t('providers.inst.cred.missing')}
                  </span>
                {/if}
                {#if inst.auth === 'keychain'}
                  <button
                    type="button"
                    class="btn subtle"
                    data-testid={`providers.inst.replaceKey.${inst.id}`}
                    onclick={() => replaceKey(inst.id)}
                  >
                    {t('providers.key.replace')}
                  </button>
                {/if}
              </dd>
            </div>
          </dl>

          <div class="actions">
            <button
              type="button"
              class="btn"
              data-testid={`providers.inst.test.${inst.id}`}
              disabled={inst.status === 'testing'}
              onclick={() => testConnection(inst.id)}
            >
              <Icon name={inst.status === 'testing' ? 'clock' : 'play'} size={14} />
              {t('providers.action.test')}
            </button>
            <button type="button" class="btn" data-testid={`providers.inst.edit.${inst.id}`} onclick={() => openEditForm(inst)}>
              <Icon name="edit" size={14} />
              {t('providers.inst.edit')}
            </button>
            {#if renaming !== inst.id}
              <button type="button" class="btn" data-testid={`providers.inst.rename.${inst.id}`} onclick={() => startRename(inst)}>
                {t('providers.inst.rename')}
              </button>
            {/if}
            <button type="button" class="btn" data-testid={`providers.inst.duplicate.${inst.id}`} onclick={() => startDuplicate(inst.id)}>
              {t('providers.inst.duplicate')}
            </button>
            <button type="button" class="btn" data-testid={`providers.inst.export.${inst.id}`} onclick={() => exportInstance(inst.id)}>
              <Icon name="download" size={14} />
              {t('providers.inst.export')}
            </button>
            {#if !inst.isDefault}
              <button type="button" class="btn subtle" data-testid={`providers.inst.makeDefault.${inst.id}`} onclick={() => makeDefault(inst.id)}>
                {t('providers.inst.makeDefault')}
              </button>
            {/if}
            <button
              type="button"
              class="btn danger"
              data-testid={`providers.inst.remove.${inst.id}`}
              onclick={() => removeInstance(inst.id)}
            >
              {confirmingRemove === inst.id ? t('providers.inst.removeConfirm') : t('common.delete')}
            </button>
          </div>

          {#if confirmingRemove === inst.id}
            <p class="hint warn" role="alert">{t('providers.inst.removeWarn')}</p>
          {/if}

          {#if duplicating === inst.id}
            <div class="dup-choice" data-testid={`providers.inst.duplicateChoice.${inst.id}`} role="group" aria-label={t('providers.duplicate.title', { name: inst.name })}>
              <p class="dup-choice-title">{t('providers.duplicate.title', { name: inst.name })}</p>
              {#if inst.auth === 'none'}
                <p class="dup-choice-note">{t('providers.duplicate.localNote')}</p>
                <div class="dup-choice-actions">
                  <button
                    type="button"
                    class="btn btn-primary"
                    data-testid={`providers.inst.duplicateLocal.${inst.id}`}
                    onclick={() => duplicate(inst.id, 'missing')}
                  >
                    {t('providers.duplicate.local')}
                  </button>
                  <button type="button" class="btn subtle" onclick={() => (duplicating = null)}>{t('common.cancel')}</button>
                </div>
              {:else}
                <p class="dup-choice-note">{t('providers.duplicate.note')}</p>
                <div class="dup-choice-actions">
                  <button
                    type="button"
                    class="btn btn-primary"
                    data-testid={`providers.inst.duplicateReuse.${inst.id}`}
                    disabled={!(inst.hasKey && inst.credRef)}
                    title={inst.hasKey && inst.credRef ? undefined : t('providers.duplicate.reuseUnavailable')}
                    onclick={() => duplicate(inst.id, 'shared-ref')}
                  >
                    {t('providers.duplicate.reuse')}
                  </button>
                  <button
                    type="button"
                    class="btn"
                    data-testid={`providers.inst.duplicateMissing.${inst.id}`}
                    onclick={() => duplicate(inst.id, 'missing')}
                  >
                    {t('providers.duplicate.needs')}
                  </button>
                  <button type="button" class="btn subtle" onclick={() => (duplicating = null)}>{t('common.cancel')}</button>
                </div>
              {/if}
            </div>
          {/if}

          {#if exportFallback?.id === inst.id}
            <div class="export-fallback" data-testid={`providers.inst.exportFallback.${inst.id}`}>
              <p class="dup-choice-note">{t('providers.export.fallbackNote')}</p>
              <pre class="export-json" data-testid={`providers.inst.exportJson.${inst.id}`}>{exportFallback.json}</pre>
            </div>
          {/if}

          {#if flash[inst.id]}
            <p class="flash" role="status" data-testid={`providers.inst.flash.${inst.id}`}>{t(flash[inst.id]!)}</p>
          {:else if inst.enabled && inst.status === 'offline'}
            <p class="hint warn"><Icon name="warning" size={13} /> {t('providers.offline.hint')}</p>
          {:else if !inst.enabled}
            <p class="hint"><Icon name="info" size={13} /> {t('providers.inst.disabledHint')}</p>
          {/if}

          <footer class="footnotes">
            <p class="note"><Icon name="info" size={13} /> {t(meta.privacy === 'local' ? 'providers.privacy.local' : 'providers.privacy.cloud')}</p>
            {#if meta.privacy === 'cloud'}
              <p class="note cost"><Icon name="warning" size={13} /> {t('providers.cost.warn')}</p>
            {/if}
          </footer>
        </article>
      {/each}
    </div>
  {/if}

  <!-- ==================== add provider ==================== -->
  <div class="add-row">
    <h2 class="section-title">{t('providers.templates.title')}</h2>
    <button type="button" class="btn btn-primary" data-testid="providers.add" onclick={() => openAddForm()}>
      <Icon name="file-plus" size={14} />
      {t('providers.add')}
    </button>
  </div>

  <div class="templates" data-testid="providers.templates">
    {#each FAMILIES as f (f.id)}
      <button
        type="button"
        class="template"
        data-testid={`providers.template.${f.id}`}
        onclick={() => openAddForm(f.id)}
      >
        <span class="card-icon"><Icon name="cpu" size={16} /></span>
        <span class="template-text">
          <span class="template-name">{t(f.nameKey)}</span>
          <span class="template-desc">{t(f.descKey)}</span>
        </span>
        <Icon name="arrow-right" size={14} />
      </button>
    {/each}
  </div>

  {#if form}
    <div class="form" data-testid="providers.form">
      <h3 class="form-title">
        {form.editingId ? t('providers.form.editTitle') : t('providers.form.addTitle')}
      </h3>

      <label class="form-row">
        <span class="form-label">{t('providers.form.name')}</span>
        <input
          type="text"
          bind:value={form.name}
          placeholder={t(`providers.example.${form.family}`)}
          data-testid="providers.form.name"
          spellcheck="false"
        />
      </label>

      <div class="form-row">
        <span class="form-label" id="providers-form-family">{t('providers.form.family')}</span>
        {#if form.editingId}
          <span class="mono family-fixed">{t(`providers.example.${form.family}`)}</span>
        {:else}
          <div class="seg" role="group" aria-labelledby="providers-form-family" data-testid="providers.form.family">
            {#each FAMILIES as f (f.id)}
              <button
                type="button"
                class="seg-btn"
                aria-pressed={form.family === f.id}
                data-testid={`providers.form.family.${f.id}`}
                onclick={() => pickFamilyInForm(f.id)}
              >
                {t(f.nameKey)}
              </button>
            {/each}
          </div>
        {/if}
      </div>

      <label class="form-row">
        <span class="form-label">{t('providers.baseUrl')}</span>
        <input
          type="text"
          class="mono"
          bind:value={form.baseUrl}
          data-testid="providers.form.baseUrl"
          spellcheck="false"
        />
      </label>

      <div class="form-row">
        <span class="form-label" id="providers-form-auth">{t('providers.form.auth')}</span>
        <div class="seg" role="group" aria-labelledby="providers-form-auth" data-testid="providers.form.auth">
          <button type="button" class="seg-btn" aria-pressed={form.auth === 'keychain'} data-testid="providers.form.auth.keychain" onclick={() => (form!.auth = 'keychain')}>
            {t('providers.form.auth.keychain')}
          </button>
          <button type="button" class="seg-btn" aria-pressed={form.auth === 'none'} data-testid="providers.form.auth.none" onclick={() => (form!.auth = 'none')}>
            {t('providers.form.auth.none')}
          </button>
        </div>
      </div>
      <p class="form-note">{t(form.auth === 'keychain' ? 'providers.key.hint' : 'providers.form.auth.noneHint')}</p>

      <div class="form-row">
        <span class="form-label" id="providers-form-disc">{t('providers.form.discovery')}</span>
        <div class="seg" role="group" aria-labelledby="providers-form-disc" data-testid="providers.form.discovery">
          <button type="button" class="seg-btn" aria-pressed={form.discovery === 'auto'} data-testid="providers.form.discovery.auto" onclick={() => (form!.discovery = 'auto')}>
            {t('providers.form.discovery.auto')}
          </button>
          <button type="button" class="seg-btn" aria-pressed={form.discovery === 'manual'} data-testid="providers.form.discovery.manual" onclick={() => (form!.discovery = 'manual')}>
            {t('providers.form.discovery.manual')}
          </button>
        </div>
      </div>

      {#if form.discovery === 'auto'}
        <div class="form-row">
          <span class="form-label">{t('providers.inst.model')}</span>
          <select bind:value={form.model} data-testid="providers.form.modelSelect" aria-label={t('providers.inst.model')}>
            {#each familyMeta(form.family).defaultModels as m (m)}
              <option value={m}>{m}</option>
            {/each}
          </select>
        </div>
      {:else}
        <label class="form-row">
          <span class="form-label">{t('providers.inst.model')}</span>
          <input
            type="text"
            class="mono"
            bind:value={form.model}
            placeholder={t('providers.form.modelPlaceholder')}
            data-testid="providers.form.modelInput"
            spellcheck="false"
          />
        </label>
      {/if}

      <div class="configure-actions">
        <button type="button" class="btn btn-primary" data-testid="providers.form.save" onclick={saveForm}>
          <Icon name="check" size={14} />
          {form.editingId ? t('providers.action.save') : t('providers.form.add')}
        </button>
        <button
          type="button"
          class="btn"
          data-testid="providers.form.test"
          onclick={() => showFlash('form', 'providers.test.ok')}
        >
          <Icon name="play" size={14} />
          {t('providers.action.test')}
        </button>
        <button type="button" class="btn" onclick={cancelForm}>{t('common.cancel')}</button>
        {#if flash.form}
          <span class="flash inline" role="status">{t(flash.form)}</span>
        {/if}
      </div>
    </div>
  {/if}

  <p class="zero" data-testid="providers.zero"><Icon name="circle-check" size={14} /> {t('providers.zero')}</p>
</section>

<style>
  .providers {
    width: min(880px, 100%);
    margin: 0 auto;
    padding: var(--space-8) var(--space-4);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .head {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
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

  .subtitle {
    margin: 0;
    color: var(--color-muted-fg);
  }

  .back {
    align-self: flex-start;
  }

  .section-title {
    margin: var(--space-2) 0 0;
    font-family: var(--font-heading);
    font-size: 16px;
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
    color: var(--color-muted-fg);
    text-transform: uppercase;
  }

  .add-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    flex-wrap: wrap;
    margin-top: var(--space-4);
  }

  .empty {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .cards {
    display: grid;
    grid-template-columns: 1fr;
    gap: var(--space-4);
    align-items: start;
  }

  .card {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: var(--shadow-card);
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .card.disabled {
    opacity: 0.65;
  }

  .card-head {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
  }

  .card-icon {
    color: var(--card-icon-fg);
    background: var(--card-icon-bg);
    padding: var(--card-icon-pad);
    border-radius: var(--card-icon-radius);
    display: inline-flex;
  }

  .card-title {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }

  .name-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
  }

  .name {
    font-family: var(--font-heading);
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
  }

  .desc {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .badge {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: var(--text-meta-size);
    padding: 0 var(--space-2);
    border-radius: 999px;
  }

  .badge.default {
    color: var(--color-primary-text);
    border: 1px solid var(--color-border);
    white-space: nowrap;
  }

  .badge.cred-shared,
  .badge.cred-missing {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: var(--text-meta-size);
    padding: 0 var(--space-2);
    border-radius: 999px;
    white-space: nowrap;
  }

  .badge.cred-shared {
    color: var(--color-primary-text);
    border: 1px dashed var(--color-border-strong);
  }

  .badge.cred-missing {
    color: var(--color-warning);
    border: 1px solid var(--color-warning);
  }

  .dup-choice {
    border: 1px dashed var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .dup-choice-title {
    margin: 0;
    font-weight: 600;
  }

  .dup-choice-note {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .dup-choice-actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .export-fallback {
    border: 1px dashed var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .export-json {
    margin: 0;
    font-family: var(--font-mono);
    font-size: var(--text-meta-size);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: var(--space-2);
    overflow-x: auto;
    user-select: all;
    white-space: pre;
  }

  .rename-row {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .rename-row input {
    min-width: 0;
    width: 200px;
  }

  .switch {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--text-meta-size);
    color: var(--color-muted-fg);
    cursor: pointer;
    white-space: nowrap;
  }

  .switch input {
    accent-color: var(--color-primary);
  }

  .status {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--text-meta-size);
    white-space: nowrap;
    padding: 2px var(--space-2);
    border-radius: 999px;
    border: 1px solid var(--color-border);
  }

  .st-connected {
    color: var(--color-success);
    border-color: var(--color-success);
  }

  .st-not_configured {
    color: var(--color-muted-fg);
  }

  .st-offline,
  .st-testing {
    color: var(--color-warning);
    border-color: var(--color-warning);
  }

  .meta {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .meta-row {
    display: grid;
    grid-template-columns: 110px 1fr;
    gap: var(--space-2);
    align-items: baseline;
  }

  .meta-row dt {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .meta-row dd {
    margin: 0;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .key-cell {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
  }

  .mono {
    font-family: var(--font-mono);
    font-size: var(--text-meta-size);
  }

  .btn.subtle {
    border-color: transparent;
    color: var(--color-primary-text);
    min-height: 26px;
    padding: 0 var(--space-2);
    font-size: var(--text-meta-size);
  }

  .btn.danger {
    color: var(--color-error);
    border-color: var(--color-border);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .flash {
    margin: 0;
    color: var(--color-success);
    font-size: var(--text-meta-size);
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .flash.inline {
    display: inline-flex;
  }

  .hint {
    margin: 0;
    font-size: var(--text-meta-size);
    display: flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--color-muted-fg);
  }

  .hint.warn {
    color: var(--color-warning);
  }

  .footnotes {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    border-top: 1px solid var(--color-border);
    padding-top: var(--space-2);
  }

  .note {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    display: flex;
    align-items: flex-start;
    gap: var(--space-1);
  }

  .note :global(svg) {
    flex: none;
    margin-top: 2px;
  }

  .note.cost :global(svg) {
    color: var(--color-warning);
  }

  .templates {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: var(--space-2);
  }

  .template {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    border: 1px dashed var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    padding: var(--space-2) var(--space-3);
    text-align: left;
    color: var(--color-fg);
    transition: border-color var(--motion-fast) var(--ease-out);
  }

  .template:hover {
    border-color: var(--color-primary);
  }

  .template > :global(svg:last-child) {
    margin-left: auto;
    color: var(--color-muted-fg);
  }

  .template-text {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .template-name {
    font-weight: 600;
  }

  .template-desc {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .form {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: var(--shadow-card);
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .form-title {
    margin: 0;
    font-family: var(--font-heading);
    font-size: 16px;
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
  }

  .form-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .form-label {
    width: 160px;
    flex: none;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .form-row input[type='text'],
  .form-row select {
    flex: 1;
    min-width: 220px;
  }

  .family-fixed {
    color: var(--color-fg);
  }

  .form-note {
    margin: calc(-1 * var(--space-2)) 0 0;
    padding-left: calc(160px + var(--space-3));
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .seg {
    display: inline-flex;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    overflow: hidden;
    background: var(--color-bg);
    flex-wrap: wrap;
  }

  .seg-btn {
    display: inline-flex;
    align-items: center;
    min-height: var(--control-h);
    padding: 0 var(--space-3);
    color: var(--color-muted-fg);
    transition: background var(--motion-fast), color var(--motion-fast);
  }

  .seg-btn + .seg-btn {
    border-left: 1px solid var(--color-border);
  }

  .seg-btn:hover {
    background: var(--color-muted);
    color: var(--color-fg);
  }

  .seg-btn[aria-pressed='true'] {
    background: var(--color-primary);
    color: var(--color-primary-fg);
  }

  .configure-actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .zero {
    margin: var(--space-2) 0 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .zero :global(svg) {
    color: var(--color-success);
  }

  @media (max-width: 560px) {
    .form-label {
      width: 100%;
    }

    .form-note {
      padding-left: 0;
    }
  }
</style>
