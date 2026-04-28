# Yacito Workbench Specification

## Purpose

`Yacito` is a desktop workbench for httpyac request files. It keeps ``.http` files` and `httpyac` as the source of truth while adding a richer UI for discovery, execution, and maintenance.

## Requirements

### Requirement: Service discovery

The system MUST discover the repo `selected .http folder` directory, load generated `.http` service files, and present their endpoints grouped by service.

#### Scenario: Load generated requests

- GIVEN `selected .http folder` exists with `.http` files
- WHEN the app starts
- THEN the sidebar lists services and endpoints parsed from those files

### Requirement: Request execution

The system MUST execute selected endpoints through `httpyac send` using the selected environment and runtime token.

#### Scenario: Execute selected endpoint

- GIVEN an endpoint, environment, and optional token are selected
- WHEN the user sends the request
- THEN the app displays stdout, stderr, and the process exit code
