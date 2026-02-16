# Proto WASM Plugin

Proto WASM plugin for installing the .NET SDK from Microsoft's official release metadata.

## What is this plugin for?

- Lists installable SDK versions by reading:
  - `https://builds.dotnet.microsoft.com/dotnet/release-metadata/releases-index.json`
  - each channel's `releases.json`
- Resolves aliases:
  - `latest`
  - `lts`
  - `sts`
  - `stable` (maps to `lts`)
  - `current` (maps to `latest`)
- Downloads runtime identifier specific SDK archives (`.tar.gz` on Unix, `.zip` on Windows)
- Exposes `dotnet` as the primary executable
- Reads `global.json` to detect pinned SDK versions

## Build

Prerequisites:

- Rust toolchain (`cargo`, `rustc`)
- WASI target: `wasm32-wasip1`

Commands:

```bash
rustup target add wasm32-wasip1
cargo build --release --target wasm32-wasip1
```

WASM output:

```text
target/wasm32-wasip1/release/dotnet_tool.wasm
```

## To use with proto locally

In a project's `.prototools` file:

```toml
dotnet = "10.0.103"

[plugins.tools]
dotnet = "file://./target/wasm32-wasip1/release/dotnet_tool.wasm"
```

The version pin (`dotnet = "..."`) must be a top-level key, not inside `[plugins.tools]`.

Install:

```bash
proto install dotnet
dotnet --info
```

### Optional plugin config

```toml
[tools.dotnet]
metadata-index-url = "https://builds.dotnet.microsoft.com/dotnet/release-metadata/releases-index.json"
include-eol-channels = false # Optional: hide EOL channels
```

## Publish for others

This repo includes a GitHub Actions workflow at `.github/workflows/release.yml`.

- On pull requests and pushes to `main`, it builds the WASM artifact.
- On tags like `v0.1.1`, it also uploads release assets:
  - `dotnet.wasm`

After creating a release tag, others can use:

```toml
dotnet = "10.0.103"

[plugins.tools]
dotnet = "https://github.com/<OWNER>/<REPO>/releases/download/v0.1.0/dotnet.wasm"
```

Or with proto's GitHub locator protocol:

```bash
proto plugin add dotnet "github://<OWNER>/<REPO>@v0.1.0"
proto install dotnet
```
