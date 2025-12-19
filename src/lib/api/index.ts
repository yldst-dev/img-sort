import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  CategoryKey,
  ClipAccelCapabilities,
  Distribution,
  PhotoDetail,
  PhotoRow,
  Progress,
  Settings,
  StartAnalysisInput,
  StartAnalysisResult,
  ScoreVector,
  StreamChunk,
  ValueStats,
} from "./types";

const PROGRESS_EVENT = "analysis://progress";
const STREAM_EVENT = "analysis://stream";

const CATEGORY_KEYS: CategoryKey[] = [
  "screenshot_document",
  "people",
  "food_cafe",
  "nature_landscape",
  "city_street_travel",
  "pets_animals",
  "products_objects",
  "other",
];

const DEFAULT_SETTINGS: Settings = {
  ollamaBaseUrl: "http://127.0.0.1:11434",
  ollamaModel: "qwen2.5vl:7b",
  ollamaThink: false,
  ollamaStream: false,
  analysisResizeEnabled: true,
  analysisMaxEdge: 768,
  analysisJpegQuality: 60,
  analysisValueEnabled: false,
  analysisConcurrency: 4,
  analysisEngine: "clip",
  clipModelDir: null,
  clipModelFile: "onnx/model_q4f16.onnx",
  clipFallbackToOllama: false,
  clipEpAuto: true,
  clipEpCoreml: true,
  clipEpCuda: false,
  clipEpRocm: false,
  clipEpDirectml: false,
  clipEpOpenvino: false,
};

const isTauri =
  typeof window !== "undefined" &&
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  !!(window as any).__TAURI_INTERNALS__;
let useMock = !isTauri;

// --- Mock fallback (dev in browser) ---
type ProgressListener = (p: Progress) => void;
type StreamListener = (p: StreamChunk) => void;
const progressListeners = new Set<ProgressListener>();
const streamListeners = new Set<StreamListener>();
let mockJobId: string | null = null;
let mockRows: PhotoRow[] = [];
let mockProgress: Progress = {
  jobId: "",
  status: "idle",
  processed: 0,
  total: 0,
  errors: 0,
};
let settings = { ...DEFAULT_SETTINGS };

function normalizeScores(raw: number[]): ScoreVector {
  const total = raw.reduce((sum, n) => sum + n, 0) || 1;
  const vals = raw.map((n) => n / total);
  const scores = Object.fromEntries(
    CATEGORY_KEYS.map((key, idx) => [key, Number(vals[idx].toFixed(4))])
  ) as ScoreVector;
  return scores;
}

function deriveCategory(scores: ScoreVector): CategoryKey {
  return (
    Object.entries(scores).reduce(
      (top, [k, v]) =>
        v > top.value ? { key: k as CategoryKey, value: v } : top,
      { key: CATEGORY_KEYS[0], value: -Infinity }
    ).key
  );
}

function generateMockRows(count: number): PhotoRow[] {
  const rows: PhotoRow[] = [];
  for (let i = 0; i < count; i++) {
    const id = crypto.randomUUID();
    const raw = CATEGORY_KEYS.map(() => Math.random() + 0.2);
    const scores = normalizeScores(raw);
    const category = deriveCategory(scores);
    const topScore = Math.max(...Object.values(scores));
    rows.push({
      id,
      fileName: `photo_${String(i + 1).padStart(4, "0")}.jpg`,
      path: `/mock/source/photo_${i + 1}.jpg`,
      category,
      topScore: Number(topScore.toFixed(4)),
      scores,
      tags: [],
      exportStatus: Math.random() > 0.9 ? "error" : "success",
      errorMessage: undefined,
    });
  }
  return rows;
}

function emitProgress(update: Partial<Progress>) {
  mockProgress = { ...mockProgress, ...update };
  progressListeners.forEach((cb) => cb(mockProgress));
}

async function invokeOrMock<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (useMock) throw new Error("mock-mode");
  return invoke<T>(cmd, args);
}

export async function getSettings(): Promise<Settings> {
  if (useMock) return settings;
  try {
    const res = await invokeOrMock<Settings>("get_settings");
    settings = res;
    return res;
  } catch {
    useMock = true;
    return settings;
  }
}

export async function setSettings(next: Settings): Promise<void> {
  settings = { ...settings, ...next };
  if (useMock) return;
  await invoke("set_settings", { settings: next });
}

export async function clearResults(): Promise<void> {
  if (useMock) {
    mockRows = [];
    return;
  }
  await invoke("clear_results");
}

export async function getClipAccelCapabilities(): Promise<ClipAccelCapabilities> {
  if (useMock) {
    return {
      cpu: { supported: true, available: true, name: "CPU" },
      coreml: {
        supported: false,
        available: false,
        name: "CoreML (Apple)",
      },
      cuda: { supported: false, available: false, name: "CUDA (NVIDIA)" },
      rocm: { supported: false, available: false, name: "ROCm (AMD)" },
      directml: { supported: false, available: false, name: "DirectML (Windows)" },
      openvino: { supported: false, available: false, name: "OpenVINO (Intel)" },
    };
  }
  return invoke("get_clip_accel_capabilities");
}

export async function getValueStats(): Promise<ValueStats> {
  if (useMock) return { valuable: 0, notValuable: 0, unknown: 0 };
  return invoke("get_value_stats");
}

export async function getClipModelFiles(): Promise<string[]> {
  if (useMock) {
    return [
      "onnx/model.onnx",
      "onnx/model_fp16.onnx",
      "onnx/model_quantized.onnx",
      "onnx/model_uint8.onnx",
      "onnx/model_q4.onnx",
      "onnx/model_q4f16.onnx",
      "onnx/model_bnb4.onnx",
    ].sort();
  }
  return invoke("get_clip_model_files");
}

export async function listOllamaModels(baseUrl: string): Promise<string[]> {
  if (useMock) {
    return ["qwen2.5vl:7b", "llama3.2:latest", "gemma2:latest"].sort();
  }
  try {
    return await invoke<string[]>("list_ollama_models", { baseUrl });
  } catch {
    return [];
  }
}

export async function testOllama(
  baseUrl: string
): Promise<{ ok: boolean; message: string }> {
  const timeoutMs = 8000;
  const timeoutPromise = new Promise<never>((_, reject) =>
    setTimeout(() => reject(new Error("연결 대기 시간 초과")), timeoutMs)
  );
  if (useMock) {
    try {
      const url = new URL(baseUrl);
      if (!url.protocol.startsWith("http")) throw new Error("Invalid protocol");
      const ok = Math.random() > 0.2;
      return ok
        ? { ok: true, message: "연결 성공(Mock)" }
        : { ok: false, message: "모의 실패: Ollama 응답 없음" };
    } catch (e) {
      return { ok: false, message: "URL 형식을 확인하세요" };
    }
  }
  try {
    const msg = await Promise.race([
      invoke<string>("test_ollama", { baseUrl }),
      timeoutPromise,
    ]);
    return { ok: true, message: msg };
  } catch (e) {
    return { ok: false, message: String(e) };
  }
}

export async function startAnalysis(
  input: StartAnalysisInput
): Promise<StartAnalysisResult> {
  if (useMock) {
    const jobId = crypto.randomUUID();
    mockJobId = jobId;
    mockProgress = {
      jobId,
      status: "running",
      processed: 0,
      total: 0,
      errors: 0,
    };
    const count = Math.floor(200 + Math.random() * 300);
    mockRows = generateMockRows(count);
    emitProgress({ total: count, status: "running" });
    const interval = setInterval(() => {
      if (mockProgress.status !== "running") {
        clearInterval(interval);
        return;
      }
      const step = Math.floor(5 + Math.random() * 12);
      const nextProcessed = Math.min(mockProgress.processed + step, mockProgress.total);
      const slice = mockRows.slice(mockProgress.processed, nextProcessed);
      const lastFile = slice.length ? slice[slice.length - 1].fileName : undefined;
      const errorsInSlice = slice.filter((r) => r.exportStatus === "error").length;
      emitProgress({
        processed: nextProcessed,
        currentFile: lastFile,
        errors: mockProgress.errors + errorsInSlice,
      });

      if (nextProcessed >= mockProgress.total) {
        emitProgress({ status: "completed", currentFile: undefined });
        clearInterval(interval);
      }
    }, 350);

    return { jobId };
  }
  return invoke("start_analysis", { input });
}

export async function cancelAnalysis(jobId: string): Promise<void> {
  if (useMock) {
    if (mockJobId === jobId) emitProgress({ status: "canceled" });
    return;
  }
  await invoke("cancel_analysis", { jobId });
}

export function onProgress(cb: ProgressListener): () => void {
  if (useMock) {
    progressListeners.add(cb);
    cb(mockProgress);
    return () => progressListeners.delete(cb);
  }
  let unlistenPromise = listen<Progress>(PROGRESS_EVENT, (event) => cb(event.payload));
  // also fetch latest snapshot
  getProgressSnapshot().then((p) => p && cb(p));
  return () => {
    unlistenPromise.then((fn) => fn());
  };
}

export function onStream(cb: StreamListener): () => void {
  if (useMock) {
    streamListeners.add(cb);
    return () => streamListeners.delete(cb);
  }
  let unlistenPromise = listen<StreamChunk>(STREAM_EVENT, (event) => cb(event.payload));
  return () => {
    unlistenPromise.then((fn) => fn());
  };
}

async function getProgressSnapshot(): Promise<Progress | null> {
  if (useMock) return mockProgress;
  try {
    const p = await invoke<Progress | null>("get_progress");
    return p;
  } catch {
    return null;
  }
}

export async function listPhotos(): Promise<PhotoRow[]> {
  if (useMock) return mockRows;
  return invoke("list_photos");
}

export async function getPhotoDetail(id: string): Promise<PhotoDetail> {
  if (useMock) {
    const found = mockRows.find((r) => r.id === id);
    if (!found) throw new Error("Photo not found");
    return found;
  }
  return invoke("get_photo_detail", { id });
}

export async function getDistribution(
  mode: "avg_score" | "count_ratio"
): Promise<Distribution> {
  if (useMock) {
    const byCategory: Record<CategoryKey, number> = Object.fromEntries(
      CATEGORY_KEYS.map((c) => [c, 0])
    ) as Record<CategoryKey, number>;

    if (mockRows.length === 0) return { mode, byCategory };

    if (mode === "count_ratio") {
      mockRows.forEach((r) => {
        byCategory[r.category] += 1;
      });
      const total = mockRows.length || 1;
      CATEGORY_KEYS.forEach((c) => {
        byCategory[c] = Number((byCategory[c] / total).toFixed(4));
      });
      return { mode, byCategory };
    }

    CATEGORY_KEYS.forEach((c) => {
      const sum = mockRows.reduce((acc, r) => acc + r.scores[c], 0);
      byCategory[c] = Number((sum / mockRows.length).toFixed(4));
    });
    return { mode, byCategory };
  }
  return invoke("get_distribution", { mode });
}

export const categories = CATEGORY_KEYS;
