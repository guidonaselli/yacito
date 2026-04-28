<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';
  import { detectLanguage, languages, translate, type Language, type TranslationKey } from '$lib/i18n';

  interface Endpoint {
    name: string;
    method: string;
    path: string;
    file: string;
  }

  interface ServiceFile {
    service: string;
    file: string;
    endpoints: Endpoint[];
  }

  interface ExecuteResult {
    stdout: string;
    stderr: string;
    exit_code: number;
  }

  interface RequestDetail {
    content: string;
  }

  interface WorkspaceCapabilities {
    generator_available: boolean;
    generator_path: string | null;
  }

  let apiHttpDir = $state('');
  let services = $state<ServiceFile[]>([]);
  let envs = $state<string[]>([]);
  let selectedEnv = $state('');
  let token = $state('');
  let selectedEndpoint = $state<Endpoint | null>(null);
  let result = $state<ExecuteResult | null>(null);
  let executing = $state(false);
  let expanded = $state<Set<string>>(new Set());
  let tokenVisible = $state(false);
  let syncEnv = $state('docker');
  let syncScope = $state('all');
  let syncing = $state(false);
  let syncResult = $state<ExecuteResult | null>(null);
  let requestContent = $state('');
  let originalRequestContent = $state('');
  let loadingRequest = $state(false);
  let choosingDir = $state(false);
  let language = $state<Language>('en');
  let capabilities = $state<WorkspaceCapabilities>({
    generator_available: false,
    generator_path: null,
  });

  function t(key: TranslationKey, params: Record<string, string | number> = {}) {
    return translate(language, key, params);
  }

  async function loadAll() {
    try {
      apiHttpDir = await invoke<string>('get_api_http_dir');
      services = await invoke<ServiceFile[]>('load_services', { apiHttpDir });
      const newEnvs = await invoke<string[]>('get_envs', { apiHttpDir });
      envs = newEnvs;
      capabilities = apiHttpDir
        ? await invoke<WorkspaceCapabilities>('get_workspace_capabilities', { apiHttpDir })
        : { generator_available: false, generator_path: null };
      if (newEnvs.length > 0 && !selectedEnv) {
        selectedEnv = newEnvs[0];
      }
      expanded = new Set(services.map(s => s.service));
    } catch (e) {
      console.error('load failed:', e);
    }
  }

  async function reloadFromDir(dir: string) {
    apiHttpDir = dir;
    selectedEndpoint = null;
    result = null;
    requestContent = '';
    originalRequestContent = '';
    services = await invoke<ServiceFile[]>('load_services', { apiHttpDir });
    const newEnvs = await invoke<string[]>('get_envs', { apiHttpDir });
    envs = newEnvs;
    capabilities = apiHttpDir
      ? await invoke<WorkspaceCapabilities>('get_workspace_capabilities', { apiHttpDir })
      : { generator_available: false, generator_path: null };
    selectedEnv = newEnvs.includes(selectedEnv) ? selectedEnv : (newEnvs[0] ?? '');
    expanded = new Set(services.map((s) => s.service));
    if (apiHttpDir) {
      await invoke('start_file_watcher', { apiHttpDir }).catch(() => {});
    }
  }

  async function chooseApiHttpDir() {
    if (choosingDir) return;
    choosingDir = true;
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: apiHttpDir || undefined,
        title: t('selectFolderTitle'),
      });
      if (!selected || Array.isArray(selected)) return;

      const saved = await invoke<string>('set_api_http_dir', { apiHttpDir: selected });
      await reloadFromDir(saved);
    } catch (e) {
      result = { stdout: '', stderr: String(e), exit_code: -1 };
    } finally {
      choosingDir = false;
    }
  }

  async function execute() {
    if (!selectedEndpoint || !selectedEnv || executing) return;
    executing = true;
    result = null;
    try {
      if (requestContent.trim()) {
        result = await invoke<ExecuteResult>('execute_raw_request', {
          apiHttpDir,
          content: requestContent,
          env: selectedEnv,
          token: token || null,
        });
      } else {
        result = await invoke<ExecuteResult>('execute_request', {
          file: selectedEndpoint.file,
          env: selectedEnv,
          name: selectedEndpoint.name,
          token: token || null,
        });
      }
    } catch (e) {
      result = { stdout: '', stderr: String(e), exit_code: -1 };
    } finally {
      executing = false;
    }
  }

  async function loadRequestDetail(ep: Endpoint) {
    loadingRequest = true;
    requestContent = '';
    originalRequestContent = '';
    try {
      const detail = await invoke<RequestDetail>('get_request_detail', {
        file: ep.file,
        name: ep.name,
      });
      requestContent = detail.content;
      originalRequestContent = detail.content;
    } catch (e) {
      result = { stdout: '', stderr: String(e), exit_code: -1 };
    } finally {
      loadingRequest = false;
    }
  }

  function resetRequestContent() {
    requestContent = originalRequestContent;
  }

  function requestDirty() {
    return requestContent !== originalRequestContent;
  }

  async function executeOriginal() {
    if (!selectedEndpoint || !selectedEnv || executing) return;
    executing = true;
    result = null;
    try {
      result = await invoke<ExecuteResult>('execute_request', {
        file: selectedEndpoint.file,
        env: selectedEnv,
        name: selectedEndpoint.name,
        token: token || null,
      });
    } catch (e) {
      result = { stdout: '', stderr: String(e), exit_code: -1 };
    } finally {
      executing = false;
    }
  }

  function selectedServiceName(): string | null {
    if (!selectedEndpoint) return null;
    return services.find((s) => s.file === selectedEndpoint?.file)?.service ?? null;
  }

  async function resync() {
    if (!apiHttpDir || syncing || !capabilities.generator_available) return;
    syncing = true;
    syncResult = null;
    const service = syncScope === 'selected' ? selectedServiceName() : null;
    try {
      syncResult = await invoke<ExecuteResult>('run_generate_http_files', {
        apiHttpDir,
        env: syncEnv,
        service,
      });
      await loadAll();
    } catch (e) {
      syncResult = { stdout: '', stderr: String(e), exit_code: -1 };
    } finally {
      syncing = false;
    }
  }

  function toggle(service: string) {
    const next = new Set(expanded);
    next.has(service) ? next.delete(service) : next.add(service);
    expanded = next;
  }

  async function select(ep: Endpoint) {
    selectedEndpoint = ep;
    result = null;
    await loadRequestDetail(ep);
  }

  function methodColor(m: string): string {
    const c: Record<string, string> = {
      GET: 'oklch(60% 0.12 230)',    /* Blue */
      POST: 'oklch(65% 0.15 150)',   /* Green */
      PUT: 'oklch(70% 0.15 70)',     /* Orange/Yellow */
      DELETE: 'oklch(60% 0.18 30)',  /* Red/Coral */
      PATCH: 'oklch(65% 0.12 180)',  /* Teal */
      HEAD: 'oklch(55% 0.12 300)',   /* Purple */
    };
    return c[m.toUpperCase()] ?? 'var(--color-text-dim)';
  }

  function isActive(ep: Endpoint) {
    return selectedEndpoint?.name === ep.name && selectedEndpoint?.file === ep.file;
  }

  function syncLabel() {
    const service = selectedServiceName();
    if (syncScope === 'selected' && service) return t('resyncService', { service });
    return t('resyncAll');
  }

  onMount(async () => {
    language = detectLanguage();
    await loadAll();
    if (apiHttpDir) {
      await invoke('start_file_watcher', { apiHttpDir }).catch(() => {});
      await listen('api-http-changed', loadAll);
    }
  });
</script>

<div class="app">
  <header>
    <div class="brand"><img src="/yacito-logo.png" alt="" aria-hidden="true" /> <span>Yacito</span></div>
    <div class="toolbar">
      <div class="folder-group">
        <label class="field folder-field">
          <span>{t('folder')}</span>
          <input type="text" value={apiHttpDir || t('noFolder')} readonly title={apiHttpDir} />
        </label>
        <button class="folder-btn" onclick={chooseApiHttpDir} disabled={choosingDir}>
          {choosingDir ? t('opening') : t('choose')}
        </button>
      </div>
      <div class="resync-group">
        <label class="field">
          <span>{t('sync')}</span>
          <select bind:value={syncEnv}>
            <option value="docker">docker</option>
            <option value="local">local</option>
          </select>
        </label>
        <label class="field">
          <span>{t('scope')}</span>
          <select bind:value={syncScope}>
            <option value="all">{t('all')}</option>
            <option value="selected" disabled={!selectedEndpoint}>{t('selected')}</option>
          </select>
        </label>
        <button
          class="resync-btn"
          onclick={resync}
          title={capabilities.generator_path ?? t('generatorHint')}
          disabled={!apiHttpDir || !capabilities.generator_available || syncing || (syncScope === 'selected' && !selectedEndpoint)}
        >
          {capabilities.generator_available ? (syncing ? t('syncing') : syncLabel()) : t('generatorUnavailable')}
        </button>
      </div>
      {#if envs.length > 0}
        <label class="field">
          <span>{t('env')}</span>
          <select bind:value={selectedEnv}>
            {#each envs as e}<option>{e}</option>{/each}
          </select>
        </label>
      {/if}
      <label class="field">
        <span>{t('language')}</span>
        <select bind:value={language}>
          {#each languages as lang}
            <option value={lang}>{lang.toUpperCase()}</option>
          {/each}
        </select>
      </label>
      <label class="field token-field">
        <span>{t('token')}</span>
        <div class="token-wrap">
          {#if tokenVisible}
            <input type="text" bind:value={token} placeholder={t('bearerToken')} />
          {:else}
            <input type="password" bind:value={token} placeholder={t('bearerToken')} />
          {/if}
          <button class="toggle-vis" onclick={() => tokenVisible = !tokenVisible} title={tokenVisible ? 'Hide token' : 'Show token'}>
            {tokenVisible ? '🙈' : '👁'}
          </button>
        </div>
      </label>
    </div>
  </header>

  <main>
    <aside>
      {#if !apiHttpDir}
        <div class="hint">{t('notFound')}</div>
      {:else if services.length === 0}
        <div class="hint">{t('noHttpFiles')}<br/><code>{apiHttpDir}</code></div>
      {/if}
      {#each services as svc}
        <div class="svc-block">
          <button class="svc-btn" onclick={() => toggle(svc.service)}>
            <span class="chevron">{expanded.has(svc.service) ? '▾' : '▸'}</span>
            {svc.service}
            <span class="badge">{svc.endpoints.length}</span>
          </button>
          {#if expanded.has(svc.service)}
            {#each svc.endpoints as ep}
              <button
                class="ep-btn"
                class:active={isActive(ep)}
                onclick={() => select(ep)}
              >
                <span class="mtag" style="color:{methodColor(ep.method)}">{ep.method}</span>
                <span class="epath">{ep.path}</span>
              </button>
            {/each}
          {/if}
        </div>
      {/each}
    </aside>

    <section class="panel">
      {#if selectedEndpoint}
        <div class="req-bar">
          <span class="mtag lg" style="color:{methodColor(selectedEndpoint.method)}">{selectedEndpoint.method}</span>
          <span class="req-path">{selectedEndpoint.path}</span>
          {#if requestDirty()}
            <span class="dirty-pill">{t('edited')}</span>
          {/if}
          <button class="send-btn secondary" onclick={executeOriginal} disabled={executing}>
            {t('original')}
          </button>
          <button class="send-btn" onclick={execute} disabled={executing}>
            {executing ? '…' : t('send')}
          </button>
        </div>
        <div class="req-name">{selectedEndpoint.name}</div>

        <div class="editor-card">
          <div class="editor-head">
            {t('requestEditor')}
            <div class="editor-actions">
              <span>{loadingRequest ? t('loading') : t('temporaryOverrides')}</span>
              <button onclick={resetRequestContent} disabled={!requestDirty() || loadingRequest}>{t('reset')}</button>
            </div>
          </div>
          <textarea
            bind:value={requestContent}
            spellcheck="false"
            disabled={loadingRequest}
            placeholder={t('loadingBlock')}
          ></textarea>
        </div>

        {#if executing}
          <div class="status-msg">{t('executing')}</div>
        {:else if result}
          <div class="resp" class:resp-err={result.exit_code !== 0}>
            <div class="resp-head">
              {t('response')}
              {#if result.exit_code === 0}
                <span class="ok">✓ {t('ok')}</span>
              {:else}
                <span class="err">✗ {t('exit', { code: result.exit_code })}</span>
              {/if}
            </div>
            {#if result.stderr}
              <pre class="out err-out">{result.stderr}</pre>
            {/if}
            {#if result.stdout}
              <pre class="out ok-out">{result.stdout}</pre>
            {:else if !result.stderr}
              <div class="status-msg">{t('noOutput')}</div>
            {/if}
          </div>
        {:else}
          <div class="status-msg idle">{t('pressSend')}</div>
        {/if}

        {#if syncResult}
          <div class="sync-log" class:sync-log-err={syncResult.exit_code !== 0}>
            <div class="sync-log-head">
              {t('syncLog')}
              {#if syncResult.exit_code === 0}
                <span class="ok">✓ {t('ok')}</span>
              {:else}
                <span class="err">✗ {t('exit', { code: syncResult.exit_code })}</span>
              {/if}
            </div>
            {#if syncResult.stderr}
              <pre class="out err-out">{syncResult.stderr}</pre>
            {/if}
            {#if syncResult.stdout}
              <pre class="out ok-out">{syncResult.stdout}</pre>
            {:else if !syncResult.stderr}
              <div class="status-msg">{t('noSyncOutput')}</div>
            {/if}
          </div>
        {/if}
      {:else}
        <div class="empty-panel">
          <div class="empty-icon">🍼</div>
          <p>{t('selectEndpoint')}</p>
          <small>{t('appName')} (yācito) — {t('selectFolderTitle')}</small>
          {#if apiHttpDir}<code style="margin-top: 10px; opacity: 0.5;">{apiHttpDir}</code>{/if}
          {#if syncResult}
            <div class="sync-log empty-sync-log" class:sync-log-err={syncResult.exit_code !== 0}>
              <div class="sync-log-head">
                {t('syncLog')}
                {#if syncResult.exit_code === 0}
                  <span class="ok">✓ {t('ok')}</span>
                {:else}
                  <span class="err">✗ {t('exit', { code: syncResult.exit_code })}</span>
                {/if}
              </div>
              {#if syncResult.stderr}
                <pre class="out err-out">{syncResult.stderr}</pre>
              {/if}
              {#if syncResult.stdout}
                <pre class="out ok-out">{syncResult.stdout}</pre>
              {/if}
            </div>
          {/if}
        </div>
      {/if}
    </section>
  </main>
</div>

<style>
  :root {
    /* Color Strategy: Restrained (Soft Baby Blue/Teal) */
    --color-bg: oklch(98% 0.01 210);
    --color-surface: oklch(100% 0 0);
    --color-surface-alt: oklch(96% 0.015 210);
    --color-border: oklch(90% 0.02 210);
    --color-text: oklch(25% 0.02 210);
    --color-text-dim: oklch(50% 0.02 210);
    
    --color-primary: oklch(70% 0.15 220);
    --color-primary-hover: oklch(65% 0.16 220);
    --color-primary-text: oklch(100% 0 0);
    
    --color-accent: oklch(75% 0.1 180); /* Teal accent */
    --color-danger: oklch(65% 0.18 30); /* Coral for errors */
    --color-success: oklch(75% 0.15 150);
    
    --radius-base: 12px;
    --radius-sm: 8px;
    --radius-lg: 16px;
    
    --spacing-base: 8px;
    --shadow-soft: 0 4px 12px -2px oklch(20% 0.02 210 / 0.05);
  }

  :global(*, *::before, *::after) { box-sizing: border-box; margin: 0; padding: 0; }
  :global(body) {
    background: var(--color-bg);
    color: var(--color-text);
    font-family: 'Inter', system-ui, -apple-system, sans-serif;
    font-size: 14px;
    height: 100vh;
    overflow: hidden;
  }

  .app { display: flex; flex-direction: column; height: 100vh; }

  header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0 20px; height: 64px;
    background: var(--color-surface);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
    z-index: 10;
  }

  .brand {
    display: flex; align-items: center; gap: 12px;
    font-weight: 700; font-size: 18px; color: var(--color-text);
    letter-spacing: -0.02em;
  }
  .brand img {
    width: 40px; height: 40px; object-fit: contain;
    filter: drop-shadow(0 2px 8px oklch(0% 0 0 / 0.1));
  }

  .toolbar { display: flex; align-items: center; gap: 16px; min-width: 0; }
  .field { display: flex; flex-direction: column; gap: 4px; }
  .field > span { 
    font-size: 10px; 
    color: var(--color-text-dim); 
    text-transform: uppercase; 
    font-weight: 600;
    letter-spacing: 0.05em; 
  }

  .folder-group, .resync-group {
    display: flex; align-items: center; gap: 12px;
    padding-right: 16px; border-right: 1px solid var(--color-border);
  }
  .folder-field input { width: 220px; font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace; font-size: 11px; }

  /* Accessibility: Focus States */
  :global(*:focus-visible) {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  /* Smooth Transitions */
  button, select, input, .ep-btn {
    transition: all 200ms cubic-bezier(0.4, 0, 0.2, 1);
  }

  select, input[type="text"], input[type="password"] {
    background-color: var(--color-surface-alt);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    color: var(--color-text);
    padding: 6px 12px;
    font-size: 13px;
    outline: none;
  }
  select:focus, input:focus { 
    border-color: var(--color-primary);
    background-color: var(--color-surface);
    box-shadow: 0 0 0 3px oklch(var(--color-primary) / 0.15);
  }

  .send-btn {
    background: var(--color-primary); color: var(--color-primary-text); 
    border: none; border-radius: var(--radius-sm);
    padding: 10px 28px; font-size: 14px; font-weight: 700; cursor: pointer;
    box-shadow: 0 4px 6px -1px oklch(var(--color-primary) / 0.2), 0 2px 4px -2px oklch(var(--color-primary) / 0.1);
  }
  .send-btn:hover:not(:disabled) { 
    background: var(--color-primary-hover); 
    transform: translateY(-1px);
    box-shadow: 0 10px 15px -3px oklch(var(--color-primary) / 0.25);
  }
  .send-btn:active:not(:disabled) { transform: translateY(0); }

  .ep-btn.active { 
    background: oklch(var(--color-primary) / 0.08); 
    color: var(--color-primary);
    font-weight: 600;
    border-left: 3px solid var(--color-primary);
    padding-left: 37px; /* Adjust for border */
  }

  .editor-card, .resp, .sync-log {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: var(--shadow-soft);
    transition: transform 200ms ease, box-shadow 200ms ease;
  }
  .editor-card:focus-within {
    box-shadow: 0 12px 20px -5px oklch(20% 0.02 210 / 0.08);
  }
  .resp.resp-err { border-color: oklch(var(--color-danger) / 0.3); }
  
  .resp-head, .sync-log-head {
    display: flex; align-items: center; justify-content: space-between;
    padding: 12px 16px; background: var(--color-surface-alt); border-bottom: 1px solid var(--color-border);
    font-size: 11px; color: var(--color-text-dim); font-weight: 700; text-transform: uppercase;
    letter-spacing: 0.05em; flex-shrink: 0;
  }
  .ok { color: var(--color-success); font-weight: 700; }
  .err { color: var(--color-danger); font-weight: 700; }

  .out {
    flex: 1; overflow: auto; padding: 16px;
    font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
    font-size: 13px; line-height: 1.6; white-space: pre-wrap; word-break: break-word;
  }
  .ok-out { color: var(--color-text); }
  .err-out {
    color: var(--color-danger); background: oklch(var(--color-danger) / 0.02);
    border-bottom: 1px solid var(--color-border); flex-shrink: 0; max-height: 30%;
  }

  .sync-log {
    display: flex; flex-direction: column; overflow: hidden;
    margin: 0 24px 20px; border: 1px solid var(--color-border); border-radius: var(--radius-lg);
    max-height: 240px; flex-shrink: 0; background: var(--color-surface);
    box-shadow: var(--shadow-soft);
  }
  .sync-log .out { max-height: 180px; flex: 0 1 auto; }

  .status-msg {
    padding: 48px 24px; text-align: center; color: var(--color-text-dim); font-style: italic;
  }
  .idle { flex: 1; display: flex; align-items: center; justify-content: center; font-size: 16px; }

  .empty-panel {
    flex: 1; display: flex; flex-direction: column;
    align-items: center; justify-content: center; gap: 16px; color: var(--color-text-dim);
  }
  .empty-icon { font-size: 64px; opacity: 0.15; filter: grayscale(1); }
  .empty-panel p { font-size: 18px; font-weight: 500; }
  .empty-panel small { font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace; font-size: 12px; }
</style>
