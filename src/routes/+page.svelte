<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';
  import { detectLanguage, languages, translate, type Language, type TranslationKey } from '$lib/i18n';

  type ThemePreference = 'system' | 'light' | 'dark';
  type PortraitPane = 'browse' | 'request' | 'response';

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

  interface PostmanImportResult {
    message: string;
    file: string;
  }

  interface PathParamField {
    key: string;
    token: string;
    value: string;
  }

  let apiHttpDir = $state('');
  let services = $state<ServiceFile[]>([]);
  let envs = $state<string[]>([]);
  let selectedEnv = $state('');
  let token = $state('');
  let selectedEndpoint = $state<Endpoint | null>(null);
  let result = $state<ExecuteResult | null>(null);

  function parseOutput(stdout: string) {
    if (!stdout) return { trace: '', body: '' };
    
    const summaryRegex = /\r?\n\d+ requests processed.*\r?\n?$/;
    let text = stdout.replace(summaryRegex, '').trim();
    
    const statusMatch = text.match(/HTTP\/\d\.\d\s+\d{3}.*(?:\r?\n|$)/g);
    if (statusMatch && statusMatch.length > 0) {
      const lastStatus = statusMatch[statusMatch.length - 1];
      const statusIdx = text.lastIndexOf(lastStatus);
      
      const afterStatus = text.substring(statusIdx);
      const splitIdx = afterStatus.search(/\r?\n\r?\n/);
      
      if (splitIdx !== -1) {
        const reqTrace = text.substring(0, statusIdx + splitIdx).trim();
        const resBody = afterStatus.substring(splitIdx).trim();
        return { trace: reqTrace, body: resBody };
      }
    }
    return { trace: '', body: text };
  }

  let parsedResult = $derived(result?.stdout ? parseOutput(result.stdout) : null);
  let hasResponse = $derived(Boolean(result && (result.stdout || result.stderr)));

  const STORAGE_PREFIX = 'yacito:';
  const defaultTraceHeight = '120px';
  const minTraceHeight = 40;
  const minResponseBodyHeight = 160;

  let settingsReady = $state(false);
  let workspaceSettingsReady = $state(false);

  function readSetting(key: string): string | null {
    if (typeof localStorage === 'undefined') return null;
    return localStorage.getItem(`${STORAGE_PREFIX}${key}`);
  }

  function writeSetting(key: string, value: string | boolean) {
    if (!settingsReady || typeof localStorage === 'undefined') return;
    localStorage.setItem(`${STORAGE_PREFIX}${key}`, String(value));
  }

  function workspaceStoragePrefix(dir = apiHttpDir): string | null {
    const normalized = dir.trim();
    return normalized ? `${STORAGE_PREFIX}workspace:${normalized}:` : null;
  }

  function readWorkspaceSetting(key: string, dir = apiHttpDir): string | null {
    if (typeof localStorage === 'undefined') return null;
    const prefix = workspaceStoragePrefix(dir);
    if (!prefix) return null;
    return localStorage.getItem(`${prefix}${key}`);
  }

  function writeWorkspaceSetting(key: string, value: string | boolean) {
    if (!workspaceSettingsReady || typeof localStorage === 'undefined') return;
    const prefix = workspaceStoragePrefix();
    if (!prefix) return;
    localStorage.setItem(`${prefix}${key}`, String(value));
  }

  function readStringArraySetting(key: string): string[] | null {
    const raw = readWorkspaceSetting(key);
    if (!raw) return null;
    try {
      const parsed = JSON.parse(raw);
      return Array.isArray(parsed) ? parsed.filter((value): value is string => typeof value === 'string') : null;
    } catch {
      return null;
    }
  }

  function restoreExpandedServices(nextServices: ServiceFile[]) {
    const saved = new Set(readStringArraySetting('expandedServices') ?? []);
    const available = nextServices.map((service) => service.service);
    const matching = available.filter((service) => saved.has(service));
    expanded = new Set(matching.length > 0 ? matching : available);
  }

  let executing = $state(false);
  let expanded = $state<Set<string>>(new Set());
  let tokenVisible = $state(false);
  let syncEnv = $state('docker');
  let syncScope = $state('all');
  let syncing = $state(false);
  let syncResult = $state<ExecuteResult | null>(null);
  let requestContent = $state('');
  let originalRequestContent = $state('');
  let pathParamValues = $state<Record<string, string>>({});
  let loadingRequest = $state(false);
  let choosingDir = $state(false);
  let language = $state<Language>('en');
  let themePreference = $state<ThemePreference>('system');
  let systemPrefersDark = $state(false);
  let activeTheme = $derived(themePreference === 'system' ? (systemPrefersDark ? 'dark' : 'light') : themePreference);
  let searchQuery = $state('');
  let importingPostman = $state(false);
  let importedFile = $state<string | null>(null);
  let traceHeight = $state(defaultTraceHeight);
  let traceOutEl = $state<HTMLPreElement | null>(null);
  let responseOutputEl = $state<HTMLDivElement | null>(null);
  let traceLabelEl = $state<HTMLDivElement | null>(null);
  let responseBodyLabelEl = $state<HTMLDivElement | null>(null);
  let traceResizeHandleEl = $state<HTMLButtonElement | null>(null);
  let portraitMode = $state(false);
  let portraitPane = $state<PortraitPane>('browse');
  let toolsExpanded = $state(false);
  let sessionExpanded = $state(false);
  let responsePretty = $state(true);
  let copiedTarget = $state<'body' | 'trace' | null>(null);

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

  let filteredServices = $derived(
    services.map(svc => {
      if (!searchQuery) return svc;
      const q = searchQuery.toLowerCase();
      if (svc.service.toLowerCase().includes(q)) return svc;
      const filteredEndpoints = svc.endpoints.filter(ep => 
        ep.path.toLowerCase().includes(q) || 
        ep.name.toLowerCase().includes(q) || 
        ep.method.toLowerCase().includes(q)
      );
      return { ...svc, endpoints: filteredEndpoints };
    }).filter(svc => svc.endpoints.length > 0)
  );

  let pathParamFields = $derived.by(() => {
    if (!selectedEndpoint) return [] as PathParamField[];
    const matches = selectedEndpoint.path.match(/\{[^{}]+\}/g) ?? [];
    const seen = new Set<string>();
    return matches
      .map((token) => {
        const key = token.slice(1, -1).trim();
        if (!key || seen.has(key)) return null;
        seen.add(key);
        return {
          key,
          token,
          value: pathParamValues[key] ?? '',
        } satisfies PathParamField;
      })
      .filter((field): field is PathParamField => Boolean(field));
  });

  let parsedResponseBody = $derived.by(() => {
    const body = parsedResult?.body;
    if (!body) return null;
    try {
      return JSON.parse(body);
    } catch {
      return null;
    }
  });

  let canPrettyResponse = $derived(parsedResponseBody !== null);

  let responseBodyOutput = $derived.by(() => {
    if (responsePretty && parsedResponseBody !== null) {
      return JSON.stringify(parsedResponseBody, null, 2);
    }
    return parsedResult?.body || result?.stdout || '';
  });

  function t(key: TranslationKey, params: Record<string, string | number> = {}) {
    return translate(language, key, params);
  }

  function normalizeCandidateToken(value: string | null | undefined): string | null {
    if (!value) return null;
    const trimmed = value.trim().replace(/^"+|"+$/g, '');
    return trimmed || null;
  }

  function extractTokenCandidate(text: string): string | null {
    const trimmed = text.trim();
    if (!trimmed) return null;

    const jwtPattern = /^[A-Za-z0-9-_]+\.[A-Za-z0-9-_]+\.[A-Za-z0-9-_+/=]+$/;
    const direct = normalizeCandidateToken(trimmed);
    if (direct && jwtPattern.test(direct)) return direct;

    try {
      const parsed = JSON.parse(trimmed);
      if (typeof parsed === 'string') {
        const tokenValue = normalizeCandidateToken(parsed);
        if (tokenValue && jwtPattern.test(tokenValue)) return tokenValue;
      }
      if (parsed && typeof parsed === 'object') {
        for (const key of ['token', 'accessToken', 'jwt', 'jwtToken']) {
          const candidate = normalizeCandidateToken((parsed as Record<string, unknown>)[key] as string | undefined);
          if (candidate && jwtPattern.test(candidate)) return candidate;
        }
      }
    } catch {
      // ignore non-JSON payloads
    }

    return null;
  }

  function adoptTokenFromResult(execResult: ExecuteResult | null, pathHint?: string | null) {
    if (!execResult || execResult.exit_code !== 0 || !execResult.stdout) return;
    const candidate = extractTokenCandidate(parseOutput(execResult.stdout).body);
    if (!candidate) return;

    const shouldAdopt =
      pathHint?.toLowerCase().includes('/login') ||
      token.trim() === '' ||
      candidate !== token.trim();

    if (shouldAdopt) {
      token = candidate;
    }
  }

  function applyPathParams(content: string) {
    let resolved = content;
    for (const field of pathParamFields) {
      const replacement = field.value.trim();
      if (!replacement) continue;
      resolved = resolved.replaceAll(field.token, replacement);
    }
    return resolved;
  }

  let resolvedRequestContent = $derived(applyPathParams(requestContent));

  let unresolvedPathParams = $derived(
    pathParamFields
      .filter((field) => resolvedRequestContent.includes(field.token))
      .map((field) => field.key)
  );

  function setPathParam(key: string, value: string) {
    pathParamValues = { ...pathParamValues, [key]: value };
  }

  function setPortraitPane(next: PortraitPane) {
    if ((next === 'request' || next === 'response') && !selectedEndpoint) return;
    if (next === 'response' && !hasResponse) return;
    portraitPane = next;
  }

  function syncPortraitMode(enabled: boolean) {
    portraitMode = enabled;
    if (!enabled) return;

    if (!selectedEndpoint) {
      portraitPane = 'browse';
      return;
    }

    if (portraitPane === 'browse') {
      portraitPane = hasResponse ? 'response' : 'request';
    }
  }

  $effect(() => writeSetting('language', language));
  $effect(() => writeSetting('themePreference', themePreference));
  $effect(() => {
    if (typeof document === 'undefined') return;
    document.documentElement.dataset.theme = activeTheme;
  });
  $effect(() => writeSetting('tokenVisible', tokenVisible));
  $effect(() => writeWorkspaceSetting('selectedEnv', selectedEnv));
  $effect(() => writeWorkspaceSetting('traceHeight', traceHeight));
  $effect(() => writeWorkspaceSetting('toolsExpanded', toolsExpanded));
  $effect(() => writeWorkspaceSetting('sessionExpanded', sessionExpanded));
  $effect(() => writeWorkspaceSetting('expandedServices', JSON.stringify([...expanded])));
  $effect(() => writeWorkspaceSetting('portraitPane', portraitPane));
  $effect(() => writeWorkspaceSetting('responsePretty', responsePretty));
  $effect(() => {
    if (portraitPane === 'response' && !hasResponse) {
      portraitPane = selectedEndpoint ? 'request' : 'browse';
    }
  });

  function hydrateWorkspaceSettings(dir: string) {
    if (!dir) {
      workspaceSettingsReady = false;
      return;
    }

    workspaceSettingsReady = false;
    traceHeight = readWorkspaceSetting('traceHeight', dir) ?? defaultTraceHeight;
    selectedEnv = readWorkspaceSetting('selectedEnv', dir) ?? '';
    toolsExpanded = readWorkspaceSetting('toolsExpanded', dir) === 'true';
    sessionExpanded = readWorkspaceSetting('sessionExpanded', dir) === 'true';
    responsePretty = readWorkspaceSetting('responsePretty', dir) !== 'false';

    const storedPortraitPane = readWorkspaceSetting('portraitPane', dir);
    if (storedPortraitPane === 'browse' || storedPortraitPane === 'request' || storedPortraitPane === 'response') {
      portraitPane = storedPortraitPane;
    } else {
      portraitPane = 'browse';
    }

    workspaceSettingsReady = true;
  }

  async function loadAll() {
    try {
      apiHttpDir = await invoke<string>('get_api_http_dir');
      hydrateWorkspaceSettings(apiHttpDir);
      services = await invoke<ServiceFile[]>('load_services', { apiHttpDir });
      const newEnvs = await invoke<string[]>('get_envs', { apiHttpDir });
      envs = newEnvs;
      capabilities = apiHttpDir
        ? await invoke<WorkspaceCapabilities>('get_workspace_capabilities', { apiHttpDir })
        : { generator_available: false, generator_path: null, internal_generator_available: false };
      if (newEnvs.length > 0 && !newEnvs.includes(selectedEnv)) {
        selectedEnv = newEnvs[0];
      }
      restoreExpandedServices(services);
    } catch (e) {
      console.error('load failed:', e);
    }
  }

  async function reloadFromDir(dir: string) {
    apiHttpDir = dir;
    hydrateWorkspaceSettings(apiHttpDir);
    selectedEndpoint = null;
    portraitPane = 'browse';
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
    restoreExpandedServices(services);
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
          content: resolvedRequestContent,
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
      adoptTokenFromResult(result, selectedEndpoint.path);
      if (result) portraitPane = 'response';
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
      pathParamValues = {};
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
      if (pathParamFields.length > 0 && originalRequestContent.trim()) {
        result = await invoke<ExecuteResult>('execute_raw_request', {
          apiHttpDir,
          content: applyPathParams(originalRequestContent),
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
      adoptTokenFromResult(result, selectedEndpoint.path);
      if (result) portraitPane = 'response';
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
    selectedEndpoint = null;
    importedFile = null; // Bring user to the main empty panel for the sync log
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

  async function createConfig() {
    if (!apiHttpDir) return;
    syncing = true;
    syncResult = null;
    selectedEndpoint = null; // Bring user to the main empty panel for the log
    try {
      const msg = await invoke<string>('create_template_config', { apiHttpDir });
      syncResult = { stdout: msg, stderr: '', exit_code: 0 };
      await loadAll();
    } catch (e) {
      syncResult = { stdout: '', stderr: String(e), exit_code: -1 };
    } finally {
      syncing = false;
    }
  }


  async function importPostmanCollection() {
    if (!apiHttpDir || importingPostman) return;
    importingPostman = true;
    syncResult = null;
    selectedEndpoint = null;
    importedFile = null;
    try {
      const selected = await open({
        directory: false,
        multiple: false,
        defaultPath: apiHttpDir,
        title: t('selectPostmanTitle'),
        filters: [{ name: 'Postman Collection', extensions: ['json'] }],
      });
      if (!selected || Array.isArray(selected)) return;

      const imported = await invoke<PostmanImportResult>('import_postman_collection', {
        apiHttpDir,
        collectionFile: selected,
      });
      importedFile = imported.file;
      syncResult = { stdout: imported.message, stderr: '', exit_code: 0 };
      await selectImportedFile(imported.file);
    } catch (e) {
      syncResult = { stdout: '', stderr: String(e), exit_code: -1 };
    } finally {
      importingPostman = false;
    }
  }

  function rememberTraceHeight(node: HTMLPreElement) {
    const observer = new ResizeObserver(() => {
      const height = Math.round(node.getBoundingClientRect().height);
      if (height >= minTraceHeight) traceHeight = `${height}px`;
    });
    observer.observe(node);
    return {
      destroy() {
        observer.disconnect();
      },
    };
  }

  function clampTraceHeight(nextHeight: number) {
    if (!responseOutputEl) return Math.max(minTraceHeight, nextHeight);
    const chromeHeight =
      (traceLabelEl?.offsetHeight ?? 0) +
      (responseBodyLabelEl?.offsetHeight ?? 0) +
      (traceResizeHandleEl?.offsetHeight ?? 0);
    const maxHeight = Math.max(minTraceHeight, responseOutputEl.clientHeight - chromeHeight - minResponseBodyHeight);
    return Math.min(Math.max(nextHeight, minTraceHeight), maxHeight);
  }

  function startTraceResize(event: PointerEvent) {
    if (!responseOutputEl) return;
    event.preventDefault();

    const startY = event.clientY;
    const storedHeight = Number.parseInt(traceHeight, 10);
    const startHeight = traceOutEl?.getBoundingClientRect().height ?? (Number.isFinite(storedHeight) ? storedHeight : 120);

    const handleMove = (moveEvent: PointerEvent) => {
      traceHeight = `${clampTraceHeight(startHeight + moveEvent.clientY - startY)}px`;
    };

    const handleUp = () => {
      window.removeEventListener('pointermove', handleMove);
      window.removeEventListener('pointerup', handleUp);
    };

    window.addEventListener('pointermove', handleMove);
    window.addEventListener('pointerup', handleUp);
  }

  function resetTraceHeight() {
    traceHeight = defaultTraceHeight;
  }

  async function copyResponsePart(target: 'body' | 'trace') {
    if (typeof navigator === 'undefined' || !navigator.clipboard) return;
    const text = target === 'trace' ? (parsedResult?.trace ?? '') : responseBodyOutput;
    if (!text) return;
    await navigator.clipboard.writeText(text);
    copiedTarget = target;
    window.setTimeout(() => {
      if (copiedTarget === target) copiedTarget = null;
    }, 1200);
  }

  async function selectImportedFile(file: string) {
    await loadAll();
    const importedService = services.find((service) => service.file === file);
    const firstEndpoint = importedService?.endpoints[0];
    if (firstEndpoint) {
      await select(firstEndpoint);
    }
  }

  function toggle(service: string) {
    const next = new Set(expanded);
    next.has(service) ? next.delete(service) : next.add(service);
    expanded = next;
  }

  async function select(ep: Endpoint) {
    selectedEndpoint = ep;
    portraitPane = 'request';
    result = null;
    await loadRequestDetail(ep);
  }

  function methodColor(m: string): string {
    const c: Record<string, string> = {
      GET: 'var(--method-get)',
      POST: 'var(--method-post)',
      PUT: 'var(--method-put)',
      DELETE: 'var(--method-delete)',
      PATCH: 'var(--method-patch)',
      HEAD: 'var(--method-head)',
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

  function isThemePreference(value: string | null): value is ThemePreference {
    return value === 'system' || value === 'light' || value === 'dark';
  }

  onMount(() => {
    const storedLanguage = readSetting('language') as Language | null;
    const storedTheme = readSetting('themePreference');
    let cleanupDark = () => {};
    let cleanupPortrait = () => {};
    language = storedLanguage && languages.includes(storedLanguage) ? storedLanguage : detectLanguage();
    themePreference = isThemePreference(storedTheme) ? storedTheme : 'system';
    if (typeof window !== 'undefined' && 'matchMedia' in window) {
      const media = window.matchMedia('(prefers-color-scheme: dark)');
      const portraitMedia = window.matchMedia('(max-width: 1180px)');
      const handleTheme = (event: MediaQueryList | MediaQueryListEvent) => {
        systemPrefersDark = event.matches;
      };
      const handlePortrait = (event: MediaQueryList | MediaQueryListEvent) => {
        syncPortraitMode(event.matches);
      };

      handleTheme(media);
      handlePortrait(portraitMedia);

      media.addEventListener('change', handleTheme);
      portraitMedia.addEventListener('change', handlePortrait);

      cleanupDark = () => media.removeEventListener('change', handleTheme);
      cleanupPortrait = () => portraitMedia.removeEventListener('change', handlePortrait);
    }
    tokenVisible = readSetting('tokenVisible') === 'true';
    settingsReady = true;
    void (async () => {
      await loadAll();
      if (apiHttpDir) {
        await invoke('start_file_watcher', { apiHttpDir }).catch(() => {});
        await listen('api-http-changed', loadAll);
      }
    })();

    return () => {
      cleanupDark();
      cleanupPortrait();
    };
  });
</script>

<div class="app" data-theme={activeTheme}>
  <header class="topbar">
    <div class="topbar-main">
      <section class="workspace-card">
        <div class="brand-lockup">
          <div class="brand-mark">
            <img src="/yacito-logo.png" alt="Yacito logo" />
          </div>
          <div class="brand-copy">
            <strong>{t('appName')}</strong>
            <span>{t('workspace')}</span>
            <small title={apiHttpDir}>{apiHttpDir || t('noFolder')}</small>
          </div>
        </div>
        <button class="primary-btn" onclick={chooseApiHttpDir} disabled={choosingDir}>
          {choosingDir ? t('opening') : t('choose')}
        </button>
      </section>

      <section class="control-panel">
        <div class="control-group control-group-wide">
          <div class="group-header">
            <div class="group-label">{t('workspaceTools')}</div>
            <button class="group-toggle" type="button" onclick={() => toolsExpanded = !toolsExpanded}>
              {portraitMode ? (toolsExpanded ? t('hide') : t('show')) : t('workspaceTools')}
            </button>
          </div>
          <div class="group-controls" class:collapsed={portraitMode && !toolsExpanded}>
            <label class="field compact-field">
              <span>{t('sync')}</span>
              <select bind:value={syncEnv}>
                <option value="docker">docker</option>
                <option value="local">local</option>
              </select>
            </label>
            <label class="field compact-field">
              <span>{t('scope')}</span>
              <select bind:value={syncScope}>
                <option value="all">{t('all')}</option>
                <option value="selected" disabled={!selectedEndpoint}>{t('selected')}</option>
              </select>
            </label>
            {#if capabilities.generator_available}
              <button
                class="secondary-btn"
                onclick={resync}
                title={capabilities.generator_path ?? t('generatorHint')}
                disabled={!apiHttpDir || syncing || (syncScope === 'selected' && !selectedEndpoint)}
              >
                {syncing ? t('syncing') : syncLabel()}
              </button>
            {:else}
              <button
                class="secondary-btn"
                onclick={createConfig}
                title={t('generatorHint')}
                disabled={!apiHttpDir || syncing}
              >
                {syncing ? t('syncing') : t('createConfig')}
              </button>
            {/if}
            <button
              class="secondary-btn"
              onclick={importPostmanCollection}
              title={t('importPostmanHint')}
              disabled={!apiHttpDir || importingPostman || syncing}
            >
              {importingPostman ? t('importingPostman') : t('importPostman')}
            </button>
          </div>
        </div>

        <div class="control-group">
          <div class="group-header">
            <div class="group-label">{t('session')}</div>
            <button class="group-toggle" type="button" onclick={() => sessionExpanded = !sessionExpanded}>
              {portraitMode ? (sessionExpanded ? t('hide') : t('show')) : t('session')}
            </button>
          </div>
          <div class="group-controls group-controls-tight" class:collapsed={portraitMode && !sessionExpanded}>
            {#if envs.length > 0}
              <label class="field compact-field">
                <span>{t('env')}</span>
                <select bind:value={selectedEnv}>
                  {#each envs as e}<option>{e}</option>{/each}
                </select>
              </label>
            {/if}
            <label class="field compact-field">
              <span>{t('language')}</span>
              <select bind:value={language}>
                {#each languages as lang}
                  <option value={lang}>{lang.toUpperCase()}</option>
                {/each}
              </select>
            </label>
            <label class="field compact-field">
              <span>{t('theme')}</span>
              <select bind:value={themePreference}>
                <option value="system">{t('themeSystem')}</option>
                <option value="light">{t('themeLight')}</option>
                <option value="dark">{t('themeDark')}</option>
              </select>
            </label>
            <label class="field token-field">
              <span>{t('token')} <em class="field-note">({token.trim() ? t('tokenLoaded') : t('tokenOptional')})</em></span>
              <div class="token-wrap">
                {#if tokenVisible}
                  <input type="text" bind:value={token} placeholder={t('bearerToken')} />
                {:else}
                  <input type="password" bind:value={token} placeholder={t('bearerToken')} />
                {/if}
                <button class="toggle-vis" onclick={() => tokenVisible = !tokenVisible} title={tokenVisible ? t('hide') : t('show')}>
                  {tokenVisible ? t('hide') : t('show')}
                </button>
              </div>
            </label>
          </div>
        </div>
      </section>
    </div>
  </header>

  {#if portraitMode}
    <nav class="portrait-nav" aria-label={t('view')}>
      <button class:active={portraitPane === 'browse'} onclick={() => setPortraitPane('browse')}>{t('browse')}</button>
      <button class:active={portraitPane === 'request'} onclick={() => setPortraitPane('request')} disabled={!selectedEndpoint}>{t('requestView')}</button>
      <button class:active={portraitPane === 'response'} onclick={() => setPortraitPane('response')} disabled={!hasResponse}>{t('response')}</button>
    </nav>
  {/if}

  <main class="shell" data-portrait-mode={portraitMode ? 'true' : 'false'} data-portrait-pane={portraitPane}>
    <aside class="sidebar">
      <div class="sidebar-top">
        <div class="sidebar-summary">
          <div class="summary-stat">
            <span>{t('servicesLabel')}</span>
            <strong>{services.length}</strong>
          </div>
          <div class="summary-stat">
            <span>{t('endpointsLabel')}</span>
            <strong>{services.reduce((acc, svc) => acc + svc.endpoints.length, 0)}</strong>
          </div>
          <div class="summary-stat summary-stat-accent">
            <span>{t('env')}</span>
            <strong>{selectedEnv || 'n/a'}</strong>
          </div>
        </div>
        {#if apiHttpDir && services.length > 0}
          <p class="search-hint">{t('searchHint')}</p>
          <div class="search-box">
            <input type="text" bind:value={searchQuery} placeholder={t('search')} />
          </div>
        {/if}
      </div>

      <div class="sidebar-body">
        {#if !apiHttpDir}
          <div class="hint">{t('notFound')}</div>
        {:else if services.length === 0}
          <div class="hint">{t('noHttpFiles')}<br /><code>{apiHttpDir}</code></div>
        {:else if filteredServices.length === 0}
          <div class="hint">{t('noSearchResults')}</div>
        {/if}

        {#each filteredServices as svc}
          <div class="svc-block">
            <button class="svc-btn" onclick={() => toggle(svc.service)}>
              <span class="chevron">{expanded.has(svc.service) || searchQuery ? '▾' : '▸'}</span>
              <span class="svc-name">{svc.service}</span>
              <span class="badge">{svc.endpoints.length}</span>
            </button>
            {#if expanded.has(svc.service) || searchQuery}
              <div class="endpoint-list">
                {#each svc.endpoints as ep}
                  <button class="ep-btn" class:active={isActive(ep)} onclick={() => select(ep)}>
                    <span class="mtag" style="color:{methodColor(ep.method)}">{ep.method}</span>
                    <span class="epath">{ep.path}</span>
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    </aside>

    <section class="panel">
      {#if selectedEndpoint}
        <div class="panel-shell">
          <div class="req-bar panel-card">
            <div class="req-heading">
              <span class="mtag lg" style="color:{methodColor(selectedEndpoint.method)}">{selectedEndpoint.method}</span>
              <div class="req-title">
                <strong>{selectedEndpoint.path}</strong>
                <span>{selectedEndpoint.name}</span>
              </div>
            </div>
            <div class="req-actions">
              {#if requestDirty()}
                <span class="dirty-pill">{t('edited')}</span>
              {/if}
              <button class="secondary-btn" onclick={executeOriginal} disabled={executing || unresolvedPathParams.length > 0}>
                {t('original')}
              </button>
              <button class="primary-btn" onclick={execute} disabled={executing || unresolvedPathParams.length > 0}>
                {executing ? t('executing') : t('send')}
              </button>
            </div>
          </div>

          <div class="workspace-grid">
            <div class="composer-stack">
              <div class="meta-card panel-card">
                <div class="meta-block">
                  <span>{t('activeService')}</span>
                  <strong>{selectedServiceName() ?? 'n/a'}</strong>
                </div>
                <div class="meta-block">
                  <span>{t('selectedFile')}</span>
                  <code title={selectedEndpoint.file}>{selectedEndpoint.file.split('/').pop() ?? selectedEndpoint.file}</code>
                </div>
                <div class="meta-block">
                  <span>{t('env')}</span>
                  <strong>{selectedEnv || 'n/a'}</strong>
                </div>
              </div>

              {#if pathParamFields.length > 0}
                <div class="request-assistant panel-card">
                  <div class="section-head">
                    <div>
                      <span>{t('requestAssistant')}</span>
                      <strong>{t('requestFlow')}</strong>
                    </div>
                    <small>{t('envBackedVarsHint')}</small>
                  </div>
                  <div class="assistant-grid">
                    <div class="assistant-section">
                      <div class="assistant-label">{t('pathParams')}</div>
                      <div class="param-grid">
                        {#each pathParamFields as field}
                          <label class="param-field">
                            <span>{field.key}</span>
                            <input
                              type="text"
                              value={field.value}
                              placeholder={t('fillValue')}
                              oninput={(event) => setPathParam(field.key, (event.currentTarget as HTMLInputElement).value)}
                            />
                          </label>
                        {/each}
                      </div>
                      {#if unresolvedPathParams.length > 0}
                        <div class="param-warning">{t('unresolvedParams')}</div>
                      {/if}
                    </div>
                    <div class="assistant-section preview-section">
                      <div class="assistant-label">{t('resolvedRequest')}</div>
                      <pre class="resolved-preview">{resolvedRequestContent}</pre>
                    </div>
                  </div>
                </div>
              {/if}

              <div class="editor-card panel-card">
                <div class="section-head">
                  <div>
                    <span>{t('requestEditor')}</span>
                    <strong>{t('requestFlow')}</strong>
                  </div>
                  <div class="editor-actions">
                    <small>{loadingRequest ? t('loading') : t('temporaryOverrides')}</small>
                    <button class="ghost-btn" onclick={resetRequestContent} disabled={!requestDirty() || loadingRequest}>{t('reset')}</button>
                  </div>
                </div>
                <textarea
                  bind:value={requestContent}
                  spellcheck="false"
                  disabled={loadingRequest}
                  placeholder={t('loadingBlock')}
                ></textarea>
              </div>
            </div>

            <div class="result-stack">
              <div class="resp panel-card" class:resp-err={result?.exit_code !== 0}>
                <div class="section-head">
                  <div>
                    <span>{t('response')}</span>
                    <strong>{t('responsePanel')}</strong>
                  </div>
                  <div class="response-head-actions">
                    {#if canPrettyResponse}
                      <button class="ghost-btn" type="button" onclick={() => responsePretty = !responsePretty}>
                        {responsePretty ? t('rawBody') : t('prettyJson')}
                      </button>
                    {/if}
                    {#if parsedResult?.trace}
                      <button class="ghost-btn" type="button" onclick={() => copyResponsePart('trace')}>
                        {copiedTarget === 'trace' ? t('copied') : t('copyTrace')}
                      </button>
                    {/if}
                    {#if result?.stdout}
                      <button class="ghost-btn" type="button" onclick={() => copyResponsePart('body')}>
                        {copiedTarget === 'body' ? t('copied') : t('copyBody')}
                      </button>
                    {/if}
                    {#if result}
                      {#if result.exit_code === 0}
                        <span class="ok">✓ {t('ok')}</span>
                      {:else}
                        <span class="err">✗ {t('exit', { code: result.exit_code })}</span>
                      {/if}
                    {/if}
                  </div>
                </div>

                {#if executing}
                  <div class="status-msg">{t('executing')}</div>
                {:else if result}
                  {#if result.stderr}
                    <pre class="out err-out">{result.stderr}</pre>
                  {/if}
                  {#if result.stdout}
                    <div class="response-output" class:has-trace={Boolean(parsedResult?.trace)} bind:this={responseOutputEl}>
                      {#if parsedResult?.trace}
                        <div class="trace-label" bind:this={traceLabelEl}>HTTP Trace</div>
                        <pre bind:this={traceOutEl} use:rememberTraceHeight class="out trace-out" style:height={traceHeight}>{parsedResult.trace}</pre>
                        <button
                          class="trace-resizer"
                          type="button"
                          bind:this={traceResizeHandleEl}
                          onpointerdown={startTraceResize}
                          ondblclick={resetTraceHeight}
                          title="Drag to resize. Double click to reset."
                          aria-label="Resize trace and response body"
                        >
                          <span></span>
                        </button>
                        <div class="trace-label response-body-label" bind:this={responseBodyLabelEl}>Response Body</div>
                      {/if}
                      <pre class="out ok-out response-body-out">{responseBodyOutput}</pre>
                    </div>
                  {:else if !result.stderr}
                    <div class="status-msg">{t('noOutput')}</div>
                  {/if}
                {:else}
                  <div class="status-msg idle">
                    <strong>{t('responsePending')}</strong>
                    <small>{t('responsePendingHint')}</small>
                  </div>
                {/if}
              </div>
            </div>
          </div>
        </div>
      {:else}
        <div class="empty-panel">
          <div class="empty-brand">
            <img src="/yacito-logo.png" alt="Yacito logo" />
          </div>
          <div class="empty-copy">
            <p>{t('selectEndpoint')}</p>
            <small>{t('selectFolderTitle')}</small>
          </div>
          {#if apiHttpDir}
            <code class="workspace-path" title={apiHttpDir}>{apiHttpDir}</code>
          {/if}
          {#if syncResult}
            <div class="sync-log panel-card" class:resp-err={syncResult.exit_code !== 0}>
              <div class="section-head">
                <div>
                  <span>{t('syncLog')}</span>
                  <strong>{t('workspaceTools')}</strong>
                </div>
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
              {#if importedFile}
                <button class="primary-btn sync-open-btn" onclick={() => selectImportedFile(importedFile!)}>{t('openImported')}</button>
              {/if}
            </div>
          {/if}
        </div>
      {/if}
    </section>
  </main>
</div>

<style>
  :global(:root) {
    color-scheme: light;
    --color-bg: oklch(98.2% 0.006 240);
    --color-surface: oklch(100% 0.002 240);
    --color-surface-alt: oklch(96.4% 0.01 240);
    --color-surface-alt-2: oklch(93.4% 0.014 240);
    --color-border: oklch(89.5% 0.016 240);
    --color-border-strong: oklch(81% 0.022 240);
    --color-text: oklch(24% 0.02 240);
    --color-text-dim: oklch(51% 0.02 240);
    --color-primary: oklch(62% 0.14 248);
    --color-primary-hover: oklch(57% 0.15 248);
    --color-primary-text: oklch(98% 0.004 240);
    --color-primary-tint: color-mix(in oklch, var(--color-primary) 12%, transparent);
    --color-primary-shadow: color-mix(in oklch, var(--color-primary) 22%, transparent);
    --color-danger: oklch(60% 0.18 28);
    --color-danger-tint: color-mix(in oklch, var(--color-danger) 10%, transparent);
    --color-danger-border: color-mix(in oklch, var(--color-danger) 30%, transparent);
    --color-success: oklch(66% 0.14 152);
    --color-code-bg: oklch(97.2% 0.008 240);
    --method-get: oklch(60% 0.12 230);
    --method-post: oklch(64% 0.15 150);
    --method-put: oklch(70% 0.14 82);
    --method-delete: oklch(60% 0.18 28);
    --method-patch: oklch(65% 0.11 188);
    --method-head: oklch(56% 0.1 300);
    --radius-sm: 10px;
    --radius-md: 16px;
    --radius-lg: 22px;
    --shadow-soft: 0 1px 2px oklch(0% 0 0 / 0.04), 0 14px 30px -18px oklch(24% 0.03 240 / 0.14);
    --shadow-focus: 0 0 0 4px var(--color-primary-tint);
  }

  :global(html[data-theme='dark']) {
    color-scheme: dark;
    --color-bg: oklch(16% 0.016 248);
    --color-surface: oklch(19% 0.018 248);
    --color-surface-alt: oklch(22.5% 0.022 248);
    --color-surface-alt-2: oklch(27% 0.024 248);
    --color-border: oklch(31% 0.026 248);
    --color-border-strong: oklch(40% 0.03 248);
    --color-text: oklch(91% 0.014 248);
    --color-text-dim: oklch(68% 0.022 248);
    --color-primary: oklch(72% 0.12 248);
    --color-primary-hover: oklch(77% 0.13 248);
    --color-primary-text: oklch(17% 0.02 248);
    --color-primary-tint: color-mix(in oklch, var(--color-primary) 16%, transparent);
    --color-primary-shadow: color-mix(in oklch, var(--color-primary) 28%, transparent);
    --color-danger: oklch(73% 0.15 28);
    --color-danger-tint: color-mix(in oklch, var(--color-danger) 12%, transparent);
    --color-danger-border: color-mix(in oklch, var(--color-danger) 32%, transparent);
    --color-success: oklch(77% 0.12 152);
    --color-code-bg: oklch(17% 0.016 248);
    --method-get: oklch(76% 0.1 230);
    --method-post: oklch(78% 0.12 150);
    --method-put: oklch(80% 0.12 82);
    --method-delete: oklch(74% 0.14 28);
    --method-patch: oklch(78% 0.1 188);
    --method-head: oklch(76% 0.1 300);
    --shadow-soft: 0 1px 1px oklch(0% 0 0 / 0.18), 0 18px 36px -24px oklch(0% 0 0 / 0.52);
  }

  :global(*, *::before, *::after) {
    box-sizing: border-box;
  }

  :global(body) {
    margin: 0;
    background: var(--color-bg);
    color: var(--color-text);
    font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    font-size: 14px;
    height: 100vh;
    overflow: hidden;
    -webkit-font-smoothing: antialiased;
    text-rendering: optimizeLegibility;
  }

  :global(button),
  :global(select),
  :global(input),
  :global(textarea) {
    font: inherit;
  }

  :global(*:focus-visible) {
    outline: none;
    box-shadow: var(--shadow-focus);
  }

  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--color-bg);
  }

  .topbar {
    padding: 16px 18px 14px;
    border-bottom: 1px solid var(--color-border);
    background: color-mix(in oklch, var(--color-surface) 88%, var(--color-bg));
  }

  .topbar-main {
    display: grid;
    grid-template-columns: minmax(320px, 1.15fr) minmax(0, 1.85fr);
    gap: 16px;
    align-items: stretch;
  }

  .workspace-card,
  .control-group {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-soft);
  }

  .workspace-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
    padding: 14px 16px;
    min-width: 0;
  }

  .brand-lockup {
    display: flex;
    align-items: center;
    gap: 14px;
    min-width: 0;
  }

  .brand-mark {
    width: 48px;
    height: 48px;
    border-radius: 16px;
    background: linear-gradient(135deg, var(--color-primary-tint), transparent 72%);
    border: 1px solid var(--color-border);
    display: grid;
    place-items: center;
    flex-shrink: 0;
  }

  .brand-mark img,
  .empty-brand img {
    width: 30px;
    height: 30px;
    object-fit: contain;
  }

  .brand-copy {
    min-width: 0;
    display: grid;
    gap: 3px;
  }

  .brand-copy strong,
  .req-title strong,
  .section-head strong,
  .meta-block strong,
  .summary-stat strong {
    font-weight: 700;
    letter-spacing: -0.02em;
  }

  .brand-copy strong {
    font-size: 17px;
  }

  .brand-copy span,
  .section-head span,
  .meta-block span,
  .summary-stat span,
  .group-label,
  .assistant-label,
  .trace-label,
  .field > span {
    font-size: 10px;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--color-text-dim);
  }

  .brand-copy small {
    color: var(--color-text-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  }

  .control-panel {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 16px;
    min-width: 0;
  }

  .control-group {
    padding: 12px 14px;
    display: grid;
    gap: 10px;
    min-width: 0;
  }

  .control-group-wide {
    min-width: 0;
  }

  .group-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .group-toggle {
    display: none;
    height: 28px;
    padding: 0 10px;
    border-radius: 999px;
    border: 1px solid var(--color-border);
    background: var(--color-surface-alt);
    color: var(--color-text-dim);
    font-size: 11px;
    font-weight: 700;
  }

  .group-controls {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    gap: 10px;
    min-width: 0;
  }

  .group-controls-tight {
    justify-content: flex-end;
  }

  .group-controls.collapsed {
    display: none;
  }

  .field {
    display: grid;
    gap: 6px;
    min-width: 0;
  }

  .field-note {
    font-style: normal;
    font-weight: 600;
    text-transform: none;
    letter-spacing: normal;
    color: var(--color-text-dim);
    opacity: 0.9;
    margin-left: 4px;
  }

  .compact-field select,
  .token-wrap input,
  .search-box input,
  .param-field input {
    width: 100%;
  }

  .compact-field {
    min-width: 100px;
  }

  select,
  input[type='text'],
  input[type='password'] {
    height: 36px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface-alt);
    color: var(--color-text);
    padding: 0 12px;
    min-width: 0;
  }

  select:hover,
  input[type='text']:hover,
  input[type='password']:hover,
  textarea:hover {
    border-color: var(--color-border-strong);
  }

  .primary-btn,
  .secondary-btn,
  .ghost-btn,
  .svc-btn,
  .ep-btn,
  .toggle-vis {
    transition: background-color 180ms ease, border-color 180ms ease, color 180ms ease, transform 180ms ease;
    cursor: pointer;
  }

  .primary-btn,
  .secondary-btn,
  .ghost-btn,
  .toggle-vis {
    height: 36px;
    border-radius: var(--radius-sm);
    padding: 0 14px;
    font-size: 12px;
    font-weight: 700;
    white-space: nowrap;
  }

  .primary-btn {
    background: var(--color-primary);
    color: var(--color-primary-text);
    border: 1px solid transparent;
    box-shadow: 0 8px 18px -12px var(--color-primary-shadow);
  }

  .primary-btn:hover:not(:disabled) {
    background: var(--color-primary-hover);
    transform: translateY(-1px);
  }

  .secondary-btn,
  .ghost-btn,
  .toggle-vis {
    background: var(--color-surface-alt);
    border: 1px solid var(--color-border);
    color: var(--color-text);
  }

  .secondary-btn:hover:not(:disabled),
  .ghost-btn:hover:not(:disabled),
  .toggle-vis:hover:not(:disabled),
  .svc-btn:hover,
  .ep-btn:hover {
    background: var(--color-surface-alt-2);
    border-color: var(--color-border-strong);
  }

  .primary-btn:disabled,
  .secondary-btn:disabled,
  .ghost-btn:disabled,
  .toggle-vis:disabled {
    opacity: 0.55;
    cursor: not-allowed;
    transform: none;
  }

  .token-field {
    min-width: min(320px, 100%);
  }

  .token-wrap {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px;
  }

  .editor-actions small,
  .section-head small,
  .search-hint {
    color: var(--color-text-dim);
    line-height: 1.45;
  }

  .shell {
    display: grid;
    grid-template-columns: clamp(280px, 24vw, 360px) minmax(0, 1fr);
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .portrait-nav {
    display: none;
    padding: 10px 14px 0;
    gap: 8px;
    background: color-mix(in oklch, var(--color-surface) 88%, var(--color-bg));
  }

  .portrait-nav button {
    flex: 1;
    height: 36px;
    border-radius: 999px;
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-text-dim);
    font-size: 12px;
    font-weight: 700;
  }

  .portrait-nav button.active {
    background: var(--color-primary-tint);
    border-color: color-mix(in oklch, var(--color-primary) 26%, var(--color-border));
    color: var(--color-text);
  }

  .portrait-nav button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .sidebar {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--color-surface);
    border-right: 1px solid var(--color-border);
    overflow: hidden;
  }

  .sidebar-top {
    padding: 16px 14px 12px;
    border-bottom: 1px solid var(--color-border);
    display: grid;
    gap: 12px;
  }

  .sidebar-summary {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
  }

  .summary-stat {
    padding: 12px 10px;
    border-radius: 14px;
    background: var(--color-surface-alt);
    border: 1px solid var(--color-border);
    display: grid;
    gap: 4px;
    min-width: 0;
  }

  .summary-stat strong {
    font-size: 18px;
  }

  .summary-stat-accent {
    background: var(--color-primary-tint);
    border-color: color-mix(in oklch, var(--color-primary) 28%, var(--color-border));
  }

  .search-box input {
    background: var(--color-bg);
  }

  .sidebar-body {
    flex: 1;
    overflow: auto;
    padding: 10px 10px 14px;
    min-height: 0;
  }

  .hint {
    padding: 26px 16px;
    color: var(--color-text-dim);
    font-size: 13px;
    line-height: 1.6;
    text-align: center;
  }

  .hint code,
  .workspace-path,
  .meta-block code {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  }

  .hint code,
  .workspace-path {
    background: var(--color-surface-alt);
    color: var(--color-text);
    padding: 6px 10px;
    border-radius: 10px;
    border: 1px solid var(--color-border);
    display: inline-block;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .svc-block {
    display: grid;
    gap: 4px;
  }

  .svc-btn,
  .ep-btn {
    width: 100%;
    border: 1px solid transparent;
    background: transparent;
    text-align: left;
    min-width: 0;
  }

  .svc-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: 12px;
    color: var(--color-text);
    font-size: 13px;
    font-weight: 700;
  }

  .svc-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chevron {
    width: 12px;
    color: var(--color-text-dim);
    text-align: center;
    flex-shrink: 0;
  }

  .badge {
    margin-left: auto;
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--color-surface-alt);
    color: var(--color-text-dim);
    font-size: 11px;
    font-weight: 700;
    border: 1px solid var(--color-border);
  }

  .endpoint-list {
    display: grid;
    gap: 3px;
  }

  .ep-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px 8px 26px;
    border-radius: 12px;
    color: var(--color-text-dim);
    font-size: 12px;
  }

  .ep-btn.active {
    background: var(--color-primary-tint);
    color: var(--color-text);
    border-color: color-mix(in oklch, var(--color-primary) 22%, var(--color-border));
  }

  .mtag {
    min-width: 42px;
    padding: 3px 7px;
    border-radius: 7px;
    border: 1px solid currentColor;
    background: var(--color-surface-alt);
    font-size: 10px;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-align: center;
    flex-shrink: 0;
  }

  .mtag.lg {
    min-width: 58px;
    padding: 4px 10px;
    font-size: 11px;
  }

  .epath,
  .req-title span,
  textarea,
  .resolved-preview,
  .out,
  .meta-block code,
  .req-title strong {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  }

  .epath,
  .meta-block code {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .panel {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background: var(--color-bg);
  }

  .panel-shell {
    flex: 1;
    min-height: 0;
    padding: 18px;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 18px;
    overflow: auto;
  }

  .panel-card,
  .request-assistant,
  .editor-card,
  .resp,
  .sync-log,
  .meta-card,
  .req-bar {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-soft);
  }

  .req-bar {
    padding: 16px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .req-heading,
  .req-actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .req-heading {
    min-width: 0;
    flex: 1;
  }

  .req-title {
    min-width: 0;
    display: grid;
    gap: 4px;
  }

  .req-title strong {
    font-size: 14px;
    line-height: 1.45;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .req-title span {
    color: var(--color-text-dim);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dirty-pill {
    padding: 5px 10px;
    border-radius: 999px;
    background: var(--color-danger-tint);
    border: 1px solid var(--color-danger-border);
    color: var(--color-danger);
    font-size: 10px;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .workspace-grid {
    display: grid;
    grid-template-columns: minmax(0, 1.12fr) minmax(340px, 0.88fr);
    gap: 18px;
    align-items: stretch;
    min-height: 0;
  }

  .composer-stack,
  .result-stack {
    display: grid;
    gap: 18px;
    min-width: 0;
    min-height: 0;
  }

  .result-stack {
    position: sticky;
    top: 18px;
    align-self: stretch;
    height: fit-content;
  }

  .meta-card {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
    padding: 14px;
  }

  .meta-block {
    min-width: 0;
    display: grid;
    gap: 6px;
    padding: 9px 12px;
    background: var(--color-surface-alt);
    border: 1px solid var(--color-border);
    border-radius: 14px;
  }

  .meta-block strong,
  .meta-block code {
    font-size: 13px;
  }

  .section-head {
    display: flex;
    align-items: start;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 16px;
    background: var(--color-surface-alt);
    border-bottom: 1px solid var(--color-border);
    border-radius: var(--radius-lg) var(--radius-lg) 0 0;
  }

  .section-head > div {
    display: grid;
    gap: 4px;
    min-width: 0;
  }

  .section-head strong {
    font-size: 15px;
  }

  .response-head-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    flex-wrap: wrap;
  }

  .editor-actions {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .ghost-btn {
    height: 32px;
    padding-inline: 12px;
  }

  .assistant-grid {
    display: grid;
    grid-template-columns: minmax(250px, 320px) minmax(0, 1fr);
  }

  .assistant-section {
    padding: 16px;
    min-width: 0;
  }

  .assistant-section + .assistant-section {
    border-left: 1px solid var(--color-border);
  }

  .param-grid {
    display: grid;
    gap: 12px;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  }

  .param-field {
    display: grid;
    gap: 6px;
  }

  .param-field span {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 12px;
    font-weight: 700;
  }

  .param-warning {
    margin-top: 12px;
    color: var(--color-danger);
    font-size: 12px;
    font-weight: 700;
  }

  .resolved-preview,
  .out,
  textarea {
    background: var(--color-code-bg);
    color: var(--color-text);
    font-size: 12px;
    line-height: 1.6;
  }

  .resolved-preview {
    margin: 0;
    padding: 14px;
    border-radius: 14px;
    overflow: auto;
    max-height: 240px;
    white-space: pre-wrap;
    word-break: break-word;
  }

  textarea {
    width: 100%;
    min-height: clamp(18rem, 40vh, 34rem);
    max-height: 72vh;
    resize: vertical;
    border: none;
    outline: none;
    padding: 16px;
  }

  .resp {
    min-height: min(70vh, 620px);
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .response-output {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }

  .response-output.has-trace {
    gap: 0;
  }

  .resp.resp-err,
  .sync-log.resp-err {
    border-color: var(--color-danger-border);
  }

  .ok {
    color: var(--color-success);
    font-weight: 800;
  }

  .err {
    color: var(--color-danger);
    font-weight: 800;
  }

  .out {
    margin: 0;
    padding: 16px;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-word;
    flex: 1;
  }

  .err-out {
    color: var(--color-danger);
    background: var(--color-danger-tint);
    flex: 0 0 auto;
    max-height: min(28vh, 220px);
    border-bottom: 1px solid var(--color-border);
  }

  .trace-label {
    padding: 8px 16px;
    background: var(--color-surface-alt);
    border-top: 1px solid var(--color-border);
    border-bottom: 1px solid var(--color-border);
  }

  .trace-out {
    flex: 0 0 auto;
    min-height: 40px;
    max-height: 60vh;
    color: var(--color-text-dim);
  }

  .trace-resizer {
    height: 14px;
    padding: 0;
    border: none;
    border-top: 1px solid var(--color-border);
    border-bottom: 1px solid var(--color-border);
    background: var(--color-surface);
    cursor: row-resize;
    flex: 0 0 auto;
    display: grid;
    place-items: center;
    transition: background-color 180ms ease, border-color 180ms ease;
  }

  .trace-resizer:hover,
  .trace-resizer:focus-visible {
    background: var(--color-surface-alt);
    border-color: var(--color-border-strong);
  }

  .trace-resizer span {
    width: 56px;
    height: 6px;
    border-radius: 999px;
    border: 1px solid var(--color-border);
    background:
      repeating-linear-gradient(
        to right,
        transparent 0 6px,
        var(--color-border-strong) 6px 10px
      ),
      var(--color-surface-alt);
    pointer-events: none;
  }

  .response-body-label {
    border-top: none;
  }

  .response-body-out {
    min-height: 0;
    flex: 1 1 auto;
  }

  .response-output.has-trace .response-body-out {
    min-height: 160px;
  }

  .status-msg {
    flex: 1;
    display: grid;
    place-items: center;
    gap: 6px;
    padding: 32px;
    text-align: center;
    color: var(--color-text-dim);
    font-size: 15px;
  }

  .status-msg strong {
    color: var(--color-text);
    font-size: 16px;
    letter-spacing: -0.01em;
  }

  .status-msg small {
    max-width: 34ch;
    line-height: 1.5;
  }

  .idle {
    font-size: 16px;
  }

  .empty-panel {
    min-height: 100%;
    padding: 28px;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: 20px;
  }

  .empty-brand {
    width: 84px;
    height: 84px;
    display: grid;
    place-items: center;
    border-radius: 28px;
    background: linear-gradient(145deg, var(--color-primary-tint), transparent 72%);
    border: 1px solid var(--color-border);
    box-shadow: var(--shadow-soft);
  }

  .empty-copy {
    display: grid;
    gap: 6px;
    text-align: center;
    max-width: 640px;
  }

  .empty-copy p {
    margin: 0;
    font-size: clamp(20px, 2vw, 28px);
    font-weight: 700;
    letter-spacing: -0.03em;
  }

  .empty-copy small {
    color: var(--color-text-dim);
    font-size: 14px;
    line-height: 1.6;
  }

  .workspace-path {
    max-width: min(100%, 780px);
  }

  .sync-log {
    width: min(960px, 100%);
    overflow: hidden;
  }

  .sync-open-btn {
    margin: 0 16px 16px auto;
  }

  @media (max-width: 1480px) {
    .control-panel {
      grid-template-columns: 1fr;
    }

    .group-controls-tight {
      justify-content: flex-start;
    }

    .workspace-grid {
      grid-template-columns: 1fr;
    }

    .result-stack {
      position: static;
    }

    .resp {
      min-height: 360px;
    }
  }

  @media (max-width: 1120px) {
    .portrait-nav {
      display: flex;
    }

    .topbar-main,
    .shell {
      grid-template-columns: 1fr;
    }

    .group-toggle {
      display: inline-flex;
      align-items: center;
      justify-content: center;
    }

    .control-group {
      gap: 8px;
      padding: 10px 12px;
    }

    .control-group .group-label {
      font-size: 10px;
    }

    .shell[data-portrait-mode='true'][data-portrait-pane='browse'] .panel {
      display: none;
    }

    .shell[data-portrait-mode='true'][data-portrait-pane='request'] .sidebar,
    .shell[data-portrait-mode='true'][data-portrait-pane='response'] .sidebar {
      display: none;
    }

    .shell[data-portrait-mode='true'] .sidebar {
      max-height: none;
      border-right: none;
    }

    .shell[data-portrait-mode='true'] .panel {
      min-height: 0;
    }

    .shell[data-portrait-mode='true'][data-portrait-pane='request'] .result-stack,
    .shell[data-portrait-mode='true'][data-portrait-pane='response'] .composer-stack {
      display: none;
    }

    .sidebar {
      max-height: 42vh;
      border-right: none;
      border-bottom: 1px solid var(--color-border);
    }

    .panel-shell {
      padding: 12px;
    }

    .sidebar-summary,
    .assistant-grid {
      grid-template-columns: 1fr;
    }

    .shell[data-portrait-mode='true'] .meta-card {
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 8px;
      padding: 12px;
    }

    .shell[data-portrait-mode='true'] .meta-block {
      padding: 8px 10px;
      gap: 4px;
    }

    .sidebar-summary {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
    }

    .summary-stat {
      flex: 1 1 140px;
      display: flex;
      align-items: baseline;
      justify-content: space-between;
      gap: 10px;
      padding: 10px 12px;
    }

    .summary-stat strong {
      font-size: 16px;
    }

    .workspace-grid {
      grid-template-columns: 1fr;
      gap: 12px;
    }

    .req-bar {
      position: sticky;
      top: 0;
      z-index: 4;
    }

    .assistant-section + .assistant-section {
      border-left: none;
      border-top: 1px solid var(--color-border);
    }
  }

  @media (max-width: 860px) {
    .topbar {
      padding: 12px;
    }

    .workspace-card,
    .control-group,
    .req-bar {
      padding: 12px;
    }

    .workspace-card,
    .req-bar {
      flex-direction: column;
      align-items: stretch;
    }

    .brand-mark {
      width: 42px;
      height: 42px;
    }

    .brand-copy strong {
      font-size: 16px;
    }

    .req-actions,
    .group-controls {
      width: 100%;
    }

    .req-actions {
      justify-content: flex-start;
      flex-wrap: wrap;
    }

    .token-field {
      min-width: 0;
      width: 100%;
    }

    .summary-stat {
      flex: 1 1 100%;
    }

    .shell[data-portrait-mode='true'] .meta-card {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }

  @media (max-width: 640px) {
    .sidebar-top,
    .sidebar-body,
    .panel-shell,
    .empty-panel {
      padding-inline: 10px;
    }

    .summary-stat strong {
      font-size: 16px;
    }

    .portrait-nav {
      padding-inline: 10px;
    }

    .section-head {
      flex-direction: column;
      align-items: stretch;
    }

    .shell[data-portrait-mode='true'] .meta-card {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }

    .shell[data-portrait-mode='true'] .param-grid {
      grid-template-columns: repeat(auto-fit, minmax(132px, 1fr));
    }

    .editor-actions {
      justify-content: flex-start;
    }

    .toggle-vis,
    .secondary-btn,
    .primary-btn {
      width: 100%;
      justify-content: center;
    }

    .token-wrap {
      grid-template-columns: 1fr;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .primary-btn,
    .secondary-btn,
    .ghost-btn,
    .toggle-vis,
    .svc-btn,
    .ep-btn {
      transition: none;
    }
  }
</style>
