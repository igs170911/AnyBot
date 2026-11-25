<!--
Release notes shared by the dev pre-release and the version-tag draft.
Workflows fill COMMIT, BUILT_AT, CHANNEL, and TAG before publishing.
-->

This build is **{{CHANNEL}}**. It is not code-signed and not notarized (spec/09 K110).

- Commit: `{{COMMIT}}`
- Built at: {{BUILT_AT}} (UTC)
- Tag: {{TAG}}

The desktop app is still a shell: it opens one window and has no chat, tasks, or account pages. The server image answers `--version` and a health check. Do not use either as a product.

## How to get past the unsigned warning

### macOS

Gatekeeper says the app is from an unidentified developer, or that it cannot be opened because it is not signed.

1. Move the app to Applications.
2. Open System Settings, then Privacy & Security.
3. In the Security section, choose Open Anyway for AnyBot, then Open on the next dialog.
4. The first launch may also ask you to confirm in Finder: control-click the app and choose Open.

`xattr -d com.apple.quarantine` removes the download flag. It does not add a signature. Prefer the System Settings path.

### Windows

SmartScreen says the publisher is unknown and Windows protected your PC.

1. On the SmartScreen dialog, choose More info.
2. Choose Run anyway.

That still does not make the file signed. Check `SHA256SUMS` before you run it.

### Linux

The `.deb` and `.rpm` are not signed. The package manager warns that the package cannot be authenticated.

1. Check the file against `SHA256SUMS` before installing:

```sh
sha256sum -c SHA256SUMS
```

On macOS the same file is checked with `shasum -a 256 -c SHA256SUMS`.

2. Install only after that line says OK. A mismatch means the file is not the one this release published.

The server image is also unsigned. `docker pull` will not show a signature. Compare the image digest printed by the workflow with the digest in this release before you run it.
