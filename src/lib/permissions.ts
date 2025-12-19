export async function askForPermissions() {
  const isTauri = Boolean((window as any).__TAURI_INTERNALS__) || Boolean((window as any).__TAURI__);
  if (!isTauri) return;
  try {
    // Try the new permissions API if available
    const maybePermission = await import("@tauri-apps/api/app");
    if ((maybePermission as any).requestPermission) {
      await (maybePermission as any).requestPermission("fs:default");
      return;
    }
  } catch {
    // ignore
  }
  // Fallback: no-op (WebKit picker will still prompt if needed)
}
