# Workspace Generators

Yacito is designed to work with any folder that behaves like a `httpyac` workspace:

- one folder selected in the UI
- one or more `.http` files anywhere inside that folder tree
- optional `http-client.env.json`
- optional `yacito.config.json` to regenerate the workspace

## Zero-config mode

If your folder already contains runnable `.http` files, you do not need any Yacito-specific setup.

Just open the folder and Yacito will:

- discover `.http` files recursively
- group them by relative path
- read environments from `http-client.env.json`
- execute requests through `httpyac`

## `yacito.config.json`

Place this file in the selected folder or its parent.

You can use either:

1. `services[]` for the built-in OpenAPI sync
2. `generator` for your own script/command

You can also keep both and choose the mode you want to use operationally.

### Built-in OpenAPI mode

```json
{
  "services": [
    {
      "name": "example-service",
      "localPort": 8080,
      "dockerPort": 5000,
      "hostVar": "exampleService",
      "openapiPath": "/v3/api-docs"
    }
  ]
}
```

### Custom generator mode

```json
{
  "generator": {
    "command": "npm",
    "args": ["run", "generate:http", "--", "{{env}}", "{{service}}"],
    "cwd": ".."
  }
}
```

## Placeholders

Supported in `command`, `args`, and `cwd`:

- `{{env}}`
- `{{service}}`
- `{{apiHttpDir}}`
- `{{repoRoot}}`

If no service is selected, `{{service}}` resolves to an empty string.

## Environment variables exposed to custom generators

- `YACITO_ENV`
- `YACITO_SERVICE`
- `YACITO_API_HTTP_DIR`
- `YACITO_REPO_ROOT`

## Auto-discovered legacy scripts

If no configured generator exists, Yacito also looks for:

- `api-http/generate-http-files.py`
- `api-http/generate-http-files.sh`
- `api-http/generate-http-files.js`
- `api-http/generate-http-files.mjs`
- `api-http/scripts/generate-http-files.py`
- `api-http/scripts/generate-http-files.sh`
- `api-http/scripts/generate-http-files.js`
- `api-http/scripts/generate-http-files.mjs`
- `api-http/.yacito/generate-http-files.py`
- `api-http/.yacito/generate-http-files.sh`
- `api-http/.yacito/generate-http-files.js`
- `api-http/.yacito/generate-http-files.mjs`

- `scripts/generate-http-files.py`
- `scripts/generate-http-files.sh`
- `scripts/generate-http-files.js`
- `scripts/generate-http-files.mjs`
- `.yacito/generate-http-files.py`
- `.yacito/generate-http-files.sh`
- `.yacito/generate-http-files.js`
- `.yacito/generate-http-files.mjs`

## Recommended workspace layout

```text
my-project/
├─ scripts/
│  └─ generate-http-files.py
├─ api-http/
│  ├─ auth/login.http
│  ├─ users/list.http
│  ├─ http-client.env.json
│  └─ yacito.config.json
```

This layout is only a recommendation. Yacito does not require this exact structure as long as the selected folder behaves like a valid `httpyac` workspace.

If you prefer the generator to live next to the workspace itself, that is fully supported and is now a first-class location.
