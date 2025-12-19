export type CategoryKey =
  | "screenshot_document"
  | "people"
  | "food_cafe"
  | "nature_landscape"
  | "city_street_travel"
  | "pets_animals"
  | "products_objects"
  | "other";

export type ScoreVector = Record<CategoryKey, number>;

export interface Settings {
  ollamaBaseUrl: string;
  ollamaModel: string;
  ollamaThink: boolean;
  ollamaStream: boolean;
  analysisResizeEnabled: boolean;
  analysisMaxEdge: number;
  analysisJpegQuality: number;
  analysisValueEnabled: boolean;
  analysisConcurrency: number;
  analysisEngine: "clip" | "ollama";
  clipModelDir?: string | null;
  clipModelFile: string;
  clipFallbackToOllama: boolean;
  clipEpAuto: boolean;
  clipEpCoreml: boolean;
  clipEpCuda: boolean;
  clipEpRocm: boolean;
  clipEpDirectml: boolean;
  clipEpOpenvino: boolean;
}

export interface ClipProviderCapability {
  supported: boolean;
  available: boolean;
  name: string;
}

export interface ClipAccelCapabilities {
  cpu: ClipProviderCapability;
  coreml: ClipProviderCapability;
  cuda: ClipProviderCapability;
  rocm: ClipProviderCapability;
  directml: ClipProviderCapability;
  openvino: ClipProviderCapability;
}

export interface StreamChunk {
  jobId: string;
  fileName: string;
  delta: string;
  done: boolean;
  reset?: boolean;
}

export interface ModelOut {
  id: string;
  path: string;
  fileName: string;
  scores: ScoreVector;
  category: CategoryKey;
  exportStatus: "pending" | "success" | "error";
  errorMessage?: string;
  tags?: string[];
  caption?: string;
  textInImage?: string;
}

export interface PhotoRow {
  id: string;
  fileName: string;
  path: string;
  category: CategoryKey;
  topScore: number;
  scores: ScoreVector;
  tags?: string[];
  exportStatus: "pending" | "success" | "error";
  errorMessage?: string;
  analysisDurationMs?: number;
  model?: string | null;
  isValuable?: boolean | null;
  valuableScore?: number | null;
}

export interface PhotoDetail extends PhotoRow {
  caption?: string;
  textInImage?: string;
  analysisLog?: string;
}

export interface ValueStats {
  valuable: number;
  notValuable: number;
  unknown: number;
}

export interface Progress {
  jobId: string;
  status: "idle" | "running" | "completed" | "canceled" | "error";
  currentFile?: string;
  processed: number;
  total: number;
  errors: number;
}

export interface Distribution {
  mode: "avg_score" | "count_ratio";
  byCategory: Record<CategoryKey, number>;
}

export interface StartAnalysisInput {
  sourceRoot: string;
  exportRoot: string;
}

export interface StartAnalysisResult {
  jobId: string;
}
