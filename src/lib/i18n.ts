export const languages = ['en', 'es'] as const;
export type Language = (typeof languages)[number];

const dictionaries = {
  en: {
    appName: 'Yacito',
    folder: 'Folder',
    noFolder: 'No folder selected',
    choose: 'Choose…',
    opening: 'Opening…',
    selectFolderTitle: 'Select folder with httpyac .http files',
    sync: 'Sync',
    scope: 'Scope',
    all: 'all',
    selected: 'selected',
    env: 'Env',
    token: 'Token',
    bearerToken: 'Bearer token...',
    notFound: 'No .http folder selected',
    noHttpFiles: 'No .http files in',
    send: 'Send ▶',
    original: 'Original',
    edited: 'edited',
    requestEditor: 'Request Editor',
    temporaryOverrides: 'temporary overrides, source file untouched',
    loading: 'loading…',
    reset: 'Reset',
    loadingBlock: 'Loading .http block...',
    executing: 'Executing...',
    response: 'Response',
    noOutput: 'No output',
    pressSend: 'Press Send to execute',
    syncLog: 'Sync Log',
    noSyncOutput: 'No sync output',
    selectEndpoint: 'Select an endpoint from the sidebar',
    resyncAll: 'Resync all',
    resyncService: 'Resync {service}',
    syncing: 'Syncing…',
    language: 'Language',
    ok: 'OK',
    exit: 'exit {code}',
    generatorUnavailable: 'Generator not detected',
    generatorHint: 'Provide a yacito.config.json or scripts/generate-http-files.py to enable Resync',
  },
  es: {
    appName: 'Yacito',
    folder: 'Carpeta',
    noFolder: 'Sin carpeta seleccionada',
    choose: 'Elegir…',
    opening: 'Abriendo…',
    selectFolderTitle: 'Seleccionar carpeta con archivos .http de httpyac',
    sync: 'Sync',
    scope: 'Alcance',
    all: 'todos',
    selected: 'seleccionado',
    env: 'Env',
    token: 'Token',
    bearerToken: 'Bearer token...',
    notFound: 'No hay carpeta .http seleccionada',
    noHttpFiles: 'No hay archivos .http en',
    send: 'Enviar ▶',
    original: 'Original',
    edited: 'editado',
    requestEditor: 'Editor de request',
    temporaryOverrides: 'overrides temporales, archivo fuente intacto',
    loading: 'cargando…',
    reset: 'Reset',
    loadingBlock: 'Cargando bloque .http...',
    executing: 'Ejecutando...',
    response: 'Respuesta',
    noOutput: 'Sin salida',
    pressSend: 'Presioná Enviar para ejecutar',
    syncLog: 'Log de Sync',
    noSyncOutput: 'Sin salida de sync',
    selectEndpoint: 'Seleccioná un endpoint desde el sidebar',
    resyncAll: 'Resync todos',
    resyncService: 'Resync {service}',
    syncing: 'Sincronizando…',
    language: 'Idioma',
    ok: 'OK',
    exit: 'exit {code}',
    generatorUnavailable: 'Generador no detectado',
    generatorHint: 'Provee un yacito.config.json o scripts/generate-http-files.py para habilitar Resync',
  },
} satisfies Record<Language, Record<string, string>>;

export type TranslationKey = keyof typeof dictionaries.en;

export function detectLanguage(): Language {
  if (typeof navigator === 'undefined') return 'en';
  const lang = navigator.language.toLowerCase();
  return lang.startsWith('es') ? 'es' : 'en';
}

export function translate(language: Language, key: TranslationKey, params: Record<string, string | number> = {}) {
  let value = dictionaries[language][key] ?? dictionaries.en[key] ?? key;
  for (const [param, replacement] of Object.entries(params)) {
    value = value.replaceAll(`{${param}}`, String(replacement));
  }
  return value;
}
