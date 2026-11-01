# Evolve Messenger

The Evolve team chat as its own desktop app (Windows, macOS, Linux). It shows the same site as the phone app, `https://messager.evolvepreneuriq.app`, signs in separately from EvolveApp, lives in the tray, tells you when unread chats go up, and updates itself from its own feed.

This repo is **separate from EvolveApp** (`evolvesystems/evolve-desktop`) on purpose: a release here can never be offered to EvolveApp users, and the other way round. The plan and the reasons are in the EIQ repo: `docs/specs/desktop/DESKTOP_MESSENGER.md` (section 8) and `docs/specs/desktop/DESKTOP_APPS_REGISTRY.md`.

## What is here

| Path | What |
|---|---|
| `src-tauri/src/main.rs` | window, tray, update check, unread poll, the two commands the page may call |
| `src-tauri/src/guard.rs` | which page may call a command, which links may be opened (std-only, unit-tested) |
| `src-tauri/src/unread.rs` | when a rising unread count becomes a notification (std-only, unit-tested) |
| `src-tauri/src/update_error.rs` | why an update failed, in plain words (std-only, unit-tested) |
| `src-tauri/src/diag.rs` | log file `evolvemessenger.log` + freeze watchdog (std-only, unit-tested) |
| `src-tauri/build.rs`, `capabilities/default.json` | the command list and what the page may call; `tests/acl.rs` fails if they drift |
| `src-tauri/tests/` | `acl.rs`, `version.rs` (run always), `release_ready.rs` (run before publishing) |
| `dist/index.html` | placeholder Tauri requires; the window loads the remote site |

## Before the first release (John)

1. Create the updater key pair on your own computer: `npx @tauri-apps/cli signer generate -w ~/evolve-messenger.key`. Keep the private file and its password to yourself; send the printed **public** key text to the PM tab. It goes into `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`) and into the Updater public key box on `/admin/app-releases` (Evolve Messenger tab).
2. In this repo's Settings, Secrets and variables, Actions: `TAURI_SIGNING_PRIVATE_KEY` (the contents of the key file) and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. For a signed Mac app also `APPLE_CERTIFICATE_BASE64`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_APP_PASSWORD`, `APPLE_TEAM_ID`, `APPLE_SIGNING_IDENTITY`.
3. Add the workflow file `.github/workflows/build-release.yml` (the text is in the EIQ repo: `docs/specs/desktop/evolve-messenger-build.yml`).
4. In this repo's Settings, Webhooks: payload URL `https://evolvepreneuriq.app/webhook/github/app-release`, content type JSON, event "Releases", the same secret as the EvolveApp webhook.
5. Replace the placeholder icons in `src-tauri/icons/` with the real ones.

The first run of the workflow should use `publish = false` (build and test only). `release_ready` refuses to publish while the public key is still the placeholder.

## Known limits (v1)

- Built and tested only on GitHub's runners; it has not been run on a real machine yet.
- Notifications show a count ("You have 3 new messages"), no names or text. They need the messenger page to be signed in and its `/api/v1/desktop/check-notifications` call to answer (not yet tested on that host).
- The window loads the phone layout; it opens at 420 x 720 and can be resized.
- No Windows code-signing certificate: Windows SmartScreen shows "Run anyway". On a Mac without Apple signing secrets, right-click Open the first time.
