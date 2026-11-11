// Every command in generate_handler! (src/main.rs) must be listed here AND granted in capabilities/default.json as "allow-<command-with-dashes>".
// Without it the messenger page gets "Command X not allowed by ACL" (the EvolveApp 2.4.21 lesson). tests/acl.rs fails when the three drift apart.
const COMMANDS: &[&str] = &[
    "report_unread",
    "report_signed_out",
    "open_external_url",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to run tauri-build");
}
