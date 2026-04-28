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
  interface WorkspaceCapabilities {
    generator_available: boolean;
    generator_path: string | null;
    internal_generator_available: boolean;
  }

  let capabilities = $state<WorkspaceCapabilities>({
    generator_available: false,
    generator_path: null,
    internal_generator_available: false,
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
        : { generator_available: false, generator_path: null, internal_generator_available: false };
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
      : { generator_available: false, generator_path: null, internal_generator_available: false };
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
    /* Evolved Soft UI Palette - Modern & Baby-Easy */
    --color-bg: oklch(99% 0.005 220);
    --color-surface: oklch(100% 0 0);
    --color-surface-alt: oklch(97% 0.01 220);
    --color-border: oklch(92% 0.015 220);
    --color-text: oklch(20% 0.02 220);
    --color-text-dim: oklch(50% 0.02 220);
    
    --color-primary: oklch(65% 0.14 225);
    --color-primary-hover: oklch(60% 0.15 225);
    --color-primary-text: oklch(100% 0 0);
    
    --color-accent: oklch(75% 0.12 180);
    --color-danger: oklch(65% 0.16 30);
    --color-success: oklch(70% 0.14 145);
    
    --radius-base: 14px;
    --radius-sm: 8px;
    --radius-lg: 20px;
    
    --spacing-base: 8px;
    /* Modern Multi-layer Shadow */
    --shadow-soft: 0 2px 4px oklch(0% 0 0 / 0.02), 0 8px 16px -4px oklch(20% 0.05 220 / 0.04);
  }

  :global(*, *::before, *::after) { box-sizing: border-box; margin: 0; padding: 0; }
  :global(body) {
    background: var(--color-bg);
    color: var(--color-text);
    font-family: 'Inter', system-ui, -apple-system, sans-serif;
    font-size: 14px;
    height: 100vh;
    overflow: hidden;
    -webkit-font-smoothing: antialiased;
  }

  .app { display: flex; flex-direction: column; height: 100vh; }

  header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0 24px; height: 72px;
    background: var(--color-surface);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
    z-index: 10;
    box-shadow: 0 1px 2px oklch(0% 0 0 / 0.02);
  }

  .brand {
    display: flex; align-items: center; gap: 14px;
    font-weight: 800; font-size: 20px; color: var(--color-text);
    letter-spacing: -0.03em;
  }
  .brand img {
    width: 44px; height: 44px; object-fit: contain;
  }

  .toolbar { display: flex; align-items: center; gap: 20px; min-width: 0; }
  .field { display: flex; flex-direction: column; gap: 4px; }
  .field > span { 
    font-size: 10px; 
    color: var(--color-text-dim); 
    text-transform: uppercase; 
    font-weight: 700;
    letter-spacing: 0.08em; 
  }

  .folder-group, .resync-group {
    display: flex; align-items: center; gap: 12px;
    padding-right: 20px; border-right: 1.5px solid var(--color-border);
  }
  .folder-field input { width: 220px; font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace; font-size: 11px; font-weight: 500; }

  /* Accessibility: Focus States */
  :global(*:focus-visible) {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  /* Modern Inputs & Buttons */
  button, select, input, .ep-btn {
    transition: all 200ms cubic-bezier(0.4, 0, 0.2, 1);
  }

  select, input[type="text"], input[type="password"] {
    background-color: var(--color-surface-alt);
    border: 1.5px solid var(--color-border);
    border-radius: var(--radius-sm);
    color: var(--color-text);
    padding: 8px 12px;
    font-size: 13px;
    outline: none;
  }
  select:focus, input:focus { 
    border-color: var(--color-primary);
    background-color: var(--color-surface);
    box-shadow: 0 0 0 4px oklch(var(--color-primary) / 0.1);
  }

  .folder-btn, .resync-btn {
    background: var(--color-surface);
    color: var(--color-text);
    border: 1.5px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 8px 16px;
    font-size: 12px;
    font-weight: 700;
    cursor: pointer;
    white-space: nowrap;
  }
  .folder-btn:hover:not(:disabled), .resync-btn:hover:not(:disabled) {
    border-color: var(--color-primary);
    color: var(--color-primary);
    background: var(--color-surface-alt);
  }

  .token-wrap { display: flex; align-items: stretch; }
  .token-wrap input { width: 160px; border-radius: var(--radius-sm) 0 0 var(--radius-sm); }
  .toggle-vis {
    background: var(--color-surface-alt);
    border: 1.5px solid var(--color-border);
    border-left: none;
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
    padding: 0 10px;
    cursor: pointer;
    font-size: 14px;
    display: flex;
    align-items: center;
  }

  main { display: flex; flex: 1; overflow: hidden; }

  aside {
    width: 320px; min-width: 240px;
    background: var(--color-surface);
    border-right: 1px solid var(--color-border);
    overflow-y: auto;
    flex-shrink: 0;
    padding: 16px 8px;
  }

  .hint { padding: 32px 24px; color: var(--color-text-dim); font-size: 13px; line-height: 1.6; text-align: center; }
  .hint code { font-size: 11px; color: var(--color-primary); background: var(--color-surface-alt); padding: 2px 6px; border-radius: 4px; font-weight: 600; }

  .svc-block { margin-bottom: 8px; }
  .svc-btn {
    display: flex; align-items: center; gap: 10px; width: 100%;
    padding: 12px 16px; background: none; border: none;
    color: var(--color-text); cursor: pointer; font-size: 14px; font-weight: 700; text-align: left;
    border-radius: var(--radius-sm);
  }
  .svc-btn:hover { background: var(--color-surface-alt); }
  .chevron { font-size: 12px; color: var(--color-text-dim); width: 16px; text-align: center; transition: transform 0.2s; }
  .badge {
    margin-left: auto; font-size: 11px; color: var(--color-text-dim);
    background: var(--color-surface-alt); border-radius: 12px; padding: 2px 10px; font-weight: 600;
  }

  .ep-btn {
    display: flex; align-items: center; gap: 12px; width: 100%;
    padding: 10px 16px 10px 36px; background: none; border: none;
    color: var(--color-text-dim); cursor: pointer; font-size: 13px; text-align: left;
    border-radius: var(--radius-sm);
    margin-top: 2px;
  }
  .ep-btn:hover { background: var(--color-bg); color: var(--color-text); }
  .ep-btn.active { 
    background: oklch(var(--color-primary) / 0.08); 
    color: var(--color-primary);
    font-weight: 700;
  }

  .mtag {
    font-size: 9px; font-weight: 800; letter-spacing: 0.08em;
    min-width: 44px; text-align: center; padding: 3px 6px;
    border-radius: 6px; background: var(--color-surface-alt);
    border: 1.5px solid currentColor;
    flex-shrink: 0;
  }
  .mtag.lg { font-size: 11px; padding: 5px 12px; min-width: 68px; }

  .epath {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    font-weight: 500;
  }

  .panel { flex: 1; display: flex; flex-direction: column; overflow: hidden; background: var(--color-bg); }

  .req-bar {
    display: flex; align-items: center; gap: 16px;
    padding: 24px 32px; background: var(--color-surface);
    border-bottom: 1px solid var(--color-border); flex-shrink: 0;
  }
  .req-path { 
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace; 
    font-size: 18px; flex: 1; color: var(--color-text); font-weight: 600;
    letter-spacing: -0.02em;
  }
  
  .send-btn {
    background: var(--color-primary); color: var(--color-primary-text); 
    border: none; border-radius: var(--radius-sm);
    padding: 10px 32px; font-size: 14px; font-weight: 800; cursor: pointer;
    box-shadow: 0 4px 12px oklch(var(--color-primary) / 0.2);
  }
  .send-btn:hover:not(:disabled) { 
    background: var(--color-primary-hover); 
    transform: translateY(-1.5px);
    box-shadow: 0 8px 20px oklch(var(--color-primary) / 0.25);
  }
  .send-btn:active:not(:disabled) { transform: translateY(0); }
  .send-btn:disabled { opacity: 0.5; cursor: not-allowed; box-shadow: none; }
  .send-btn.secondary { background: var(--color-surface-alt); color: var(--color-text); box-shadow: none; border: 1.5px solid var(--color-border); }
  .send-btn.secondary:hover:not(:disabled) { border-color: var(--color-primary); color: var(--color-primary); }

  .dirty-pill {
    color: var(--color-danger); border: 1.5px solid oklch(var(--color-danger) / 0.2); background: oklch(var(--color-danger) / 0.05);
    border-radius: 999px; padding: 3px 12px; font-size: 11px; font-weight: 700;
    text-transform: uppercase; letter-spacing: 0.05em;
  }

  .req-name {
    padding: 12px 32px 16px; font-size: 13px; color: var(--color-text-dim);
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    background: var(--color-surface); flex-shrink: 0;
    font-weight: 500;
  }

  .editor-card, .resp, .sync-log {
    display: flex; flex-direction: column;
    margin: 24px 32px 0; border: 1px solid var(--color-border); border-radius: var(--radius-lg);
    background: var(--color-surface); overflow: hidden;
    box-shadow: var(--shadow-soft);
  }
  .editor-card:focus-within {
    box-shadow: 0 12px 30px -10px oklch(20% 0.1 220 / 0.08);
    border-color: var(--color-primary);
  }

  .editor-head, .resp-head, .sync-log-head {
    display: flex; align-items: center; justify-content: space-between;
    padding: 14px 20px; background: var(--color-surface-alt); border-bottom: 1px solid var(--color-border);
    font-size: 11px; color: var(--color-text-dim); font-weight: 800; text-transform: uppercase;
    letter-spacing: 0.1em;
  }
  .editor-actions { display: flex; align-items: center; gap: 12px; font-weight: 600; text-transform: none; letter-spacing: normal; }
  .editor-actions button {
    background: var(--color-surface); color: var(--color-text); border: 1.5px solid var(--color-border); border-radius: 6px;
    padding: 4px 12px; cursor: pointer; font-size: 12px; font-weight: 700;
  }
  .editor-actions button:hover:not(:disabled) { border-color: var(--color-primary); color: var(--color-primary); }

  textarea {
    width: 100%; height: 280px; resize: vertical; min-height: 120px; max-height: 45vh;
    background: var(--color-surface); color: var(--color-text); border: none; outline: none; padding: 20px;
    font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
    font-size: 13px; line-height: 1.7;
  }

  .resp { flex: 1; margin: 24px 32px; }
  .resp.resp-err { border-color: oklch(var(--color-danger) / 0.3); }
  
  .ok { color: var(--color-success); font-weight: 800; }
  .err { color: var(--color-danger); font-weight: 800; }

  .out {
    flex: 1; overflow: auto; padding: 20px;
    font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
    font-size: 13px; line-height: 1.7; white-space: pre-wrap; word-break: break-word;
  }
  .ok-out { color: var(--color-text); }
  .err-out {
    color: var(--color-danger); background: oklch(var(--color-danger) / 0.02);
    border-bottom: 1px solid var(--color-border); flex-shrink: 0; max-height: 35%;
  }

  .sync-log { margin: 0 32px 24px; max-height: 260px; flex-shrink: 0; }
  .sync-log .out { max-height: 200px; flex: 0 1 auto; }

  .status-msg {
    padding: 64px 32px; text-align: center; color: var(--color-text-dim); font-style: italic; font-size: 15px;
  }
  .idle { flex: 1; display: flex; align-items: center; justify-content: center; font-size: 18px; font-weight: 500; opacity: 0.6; }

  .empty-panel {
    flex: 1; display: flex; flex-direction: column;
    align-items: center; justify-content: center; gap: 20px; color: var(--color-text-dim);
  }
  .empty-icon { font-size: 80px; opacity: 0.2; filter: drop-shadow(0 10px 20px oklch(0% 0 0 / 0.1)); }
  .empty-panel p { font-size: 20px; font-weight: 600; color: var(--color-text); }
  .empty-panel small { font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace; font-size: 13px; opacity: 0.7; }
</style>
