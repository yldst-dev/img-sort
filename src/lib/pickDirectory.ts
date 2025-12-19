import { isTauriRuntime } from "./tauriRuntime";

export async function pickDirectory(current?: string) {
  if (isTauriRuntime()) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selection = await open({
      directory: true,
      multiple: false,
      defaultPath: current || undefined,
      title: "폴더 선택",
    });

    if (typeof selection === "string") return selection;
    if (Array.isArray(selection) && typeof selection[0] === "string") return selection[0];
    return null;
  }

  const picker = (window as any).showDirectoryPicker;
  if (typeof picker === "function") {
    const handle = await picker();
    return handle?.name ?? null;
  }

  const value = window.prompt("경로를 입력하거나 붙여넣으세요", current ?? "");
  return value || null;
}

