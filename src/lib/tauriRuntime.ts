export function isTauriRuntime() {
  return (
    typeof window !== "undefined" &&
    (Boolean((window as any).__TAURI_INTERNALS__) ||
      Boolean((window as any).__TAURI__) ||
      Boolean(import.meta.env.TAURI_PLATFORM))
  );
}

