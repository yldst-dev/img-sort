import {
  createContext,
  ReactNode,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  categories,
  cancelAnalysis,
  clearResults as apiClearResults,
  getClipAccelCapabilities,
  getClipModelFiles,
  getDistribution,
  getPhotoDetail,
  getSettings as apiGetSettings,
  getValueStats,
  listOllamaModels,
  listPhotos,
  onProgress,
  onStream,
  setSettings as apiSetSettings,
  startAnalysis,
  testOllama,
} from "../../lib/api";
import {
  CategoryKey,
  ClipAccelCapabilities,
  Distribution,
  PhotoDetail,
  PhotoRow,
  Progress,
  StreamChunk,
  ValueStats,
} from "../../lib/api/types";
import { toast } from "../../components/toast/toastBus";

interface AnalysisState {
  sourceRoot: string;
  exportRoot: string;
  settingsBaseUrl: string;
  settingsModel: string;
  settingsThink: boolean;
  settingsStream: boolean;
  settingsResizeEnabled: boolean;
  settingsMaxEdge: number;
  settingsJpegQuality: number;
  settingsValueEnabled: boolean;
  settingsConcurrency: number;
  settingsEngine: "clip" | "ollama";
  settingsClipModelDir: string;
  settingsClipModelFile: string;
  settingsClipFallback: boolean;
  settingsClipEpAuto: boolean;
  settingsClipEpCoreml: boolean;
  settingsClipEpCuda: boolean;
  settingsClipEpRocm: boolean;
  settingsClipEpDirectml: boolean;
  settingsClipEpOpenvino: boolean;
  clipAccelCaps: ClipAccelCapabilities | null;
  clipModelFiles: string[];
  availableModels: string[];
  jobElapsedMs: number | null;
  progress: Progress;
  photos: PhotoRow[];
  distributionAvg: Distribution | null;
  distributionCount: Distribution | null;
  valueStats: ValueStats | null;
  categoryFilter: CategoryKey | "all";
  selectedPhoto: PhotoDetail | null;
  loadingDetail: boolean;
  starting: boolean;
  streamPanel: { fileName: string | null; text: string; isOpen: boolean };
}

interface AnalysisActions {
  updateSourceRoot: (v: string) => void;
  updateExportRoot: (v: string) => void;
  saveSettings: (next: {
    baseUrl: string;
    model: string;
    think: boolean;
    stream: boolean;
    resizeEnabled: boolean;
    maxEdge: number;
    jpegQuality: number;
    valueEnabled: boolean;
    concurrency: number;
    engine: "clip" | "ollama";
    clipModelDir: string;
    clipModelFile: string;
    clipFallbackToOllama: boolean;
    clipEpAuto: boolean;
    clipEpCoreml: boolean;
    clipEpCuda: boolean;
    clipEpRocm: boolean;
    clipEpDirectml: boolean;
    clipEpOpenvino: boolean;
  }) => void;
  testConnection: (baseUrl: string) => Promise<void>;
  startAnalysisNow: () => Promise<void>;
  cancelCurrent: () => Promise<void>;
  setCategoryFilter: (c: CategoryKey | "all") => void;
  loadPhotoDetail: (id: string) => Promise<void>;
  closeDetail: () => void;
  resetResults: () => Promise<void>;
}

type AnalysisContextValue = AnalysisState & AnalysisActions;

const AnalysisContext = createContext<AnalysisContextValue | null>(null);

export function AnalysisProvider({ children }: { children: ReactNode }) {
  const [sourceRoot, setSourceRoot] = useState<string>("");
  const [exportRoot, setExportRoot] = useState<string>("");
  const [settingsBaseUrl, setSettingsBaseUrl] =
    useState<string>("http://127.0.0.1:11434");
  const [settingsModel, setSettingsModel] = useState<string>("qwen2.5vl:7b");
  const [settingsThink, setSettingsThink] = useState<boolean>(false);
  const [settingsStream, setSettingsStream] = useState<boolean>(false);
  const [settingsResizeEnabled, setSettingsResizeEnabled] =
    useState<boolean>(true);
  const [settingsMaxEdge, setSettingsMaxEdge] = useState<number>(768);
  const [settingsJpegQuality, setSettingsJpegQuality] = useState<number>(60);
  const [settingsValueEnabled, setSettingsValueEnabled] = useState<boolean>(false);
  const [settingsConcurrency, setSettingsConcurrency] = useState<number>(4);
  const [settingsEngine, setSettingsEngine] = useState<"clip" | "ollama">("clip");
  const [settingsClipModelDir, setSettingsClipModelDir] = useState<string>("");
  const [settingsClipModelFile, setSettingsClipModelFile] =
    useState<string>("onnx/model_q4f16.onnx");
  const [settingsClipFallback, setSettingsClipFallback] = useState<boolean>(true);
  const [settingsClipEpAuto, setSettingsClipEpAuto] = useState<boolean>(true);
  const [settingsClipEpCoreml, setSettingsClipEpCoreml] = useState<boolean>(true);
  const [settingsClipEpCuda, setSettingsClipEpCuda] = useState<boolean>(false);
  const [settingsClipEpRocm, setSettingsClipEpRocm] = useState<boolean>(false);
  const [settingsClipEpDirectml, setSettingsClipEpDirectml] = useState<boolean>(false);
  const [settingsClipEpOpenvino, setSettingsClipEpOpenvino] = useState<boolean>(false);
  const [clipAccelCaps, setClipAccelCaps] = useState<ClipAccelCapabilities | null>(null);
  const [clipModelFiles, setClipModelFiles] = useState<string[]>([]);
  const [availableModels, setAvailableModels] = useState<string[]>([]);
  const [progress, setProgress] = useState<Progress>({
    jobId: "",
    status: "idle",
    processed: 0,
    total: 0,
    errors: 0,
  });
  const [photos, setPhotos] = useState<PhotoRow[]>([]);
  const [distributionAvg, setDistributionAvg] = useState<Distribution | null>(null);
  const [distributionCount, setDistributionCount] = useState<Distribution | null>(null);
  const [valueStats, setValueStats] = useState<ValueStats | null>(null);
  const [categoryFilter, setCategoryFilterState] = useState<CategoryKey | "all">("all");
  const [selectedPhoto, setSelectedPhoto] = useState<PhotoDetail | null>(null);
  const [loadingDetail, setLoadingDetail] = useState(false);
  const [starting, setStarting] = useState(false);
  const [jobElapsedMs, setJobElapsedMs] = useState<number | null>(null);
  const jobStartedAtRef = useRef<number | null>(null);
  const [streamPanel, setStreamPanel] = useState<{
    fileName: string | null;
    text: string;
    isOpen: boolean;
  }>({ fileName: null, text: "", isOpen: false });

  useEffect(() => {
    apiGetSettings().then((s) => {
      setSettingsBaseUrl(s.ollamaBaseUrl);
      setSettingsModel(s.ollamaModel);
      setSettingsThink(s.ollamaThink);
      setSettingsStream(s.ollamaStream);
      setSettingsResizeEnabled(s.analysisResizeEnabled);
      setSettingsMaxEdge(s.analysisMaxEdge);
      setSettingsJpegQuality(s.analysisJpegQuality);
      setSettingsValueEnabled(Boolean(s.analysisValueEnabled));
      setSettingsConcurrency(
        Number.isFinite(s.analysisConcurrency) ? Number(s.analysisConcurrency) : 4
      );
      setSettingsEngine(s.analysisEngine);
      setSettingsClipModelDir(s.clipModelDir ? String(s.clipModelDir) : "");
      setSettingsClipModelFile(s.clipModelFile || "onnx/model_q4f16.onnx");
      setSettingsClipFallback(Boolean(s.clipFallbackToOllama));
      setSettingsClipEpAuto(Boolean(s.clipEpAuto));
      setSettingsClipEpCoreml(Boolean(s.clipEpCoreml));
      setSettingsClipEpCuda(Boolean(s.clipEpCuda));
      setSettingsClipEpRocm(Boolean(s.clipEpRocm));
      setSettingsClipEpDirectml(Boolean(s.clipEpDirectml));
      setSettingsClipEpOpenvino(Boolean(s.clipEpOpenvino));
    });
    getClipAccelCapabilities()
      .then((caps) => setClipAccelCaps(caps))
      .catch(() => setClipAccelCaps(null));
    getClipModelFiles()
      .then((files) => setClipModelFiles(files))
      .catch(() => setClipModelFiles([]));
  }, []);

  useEffect(() => {
    const unsub = onProgress((p) => {
      setProgress(p);
      if (jobStartedAtRef.current != null) setJobElapsedMs(Date.now() - jobStartedAtRef.current);
      if (p.status === "completed") {
        refreshListAndDistributions();
        if (jobStartedAtRef.current != null) setJobElapsedMs(Date.now() - jobStartedAtRef.current);
        toast.success("분석 완료");
        setStreamPanel((prev) => ({ ...prev, isOpen: false }));
      } else if (p.status === "canceled") {
        toast.warning("분석이 취소되었습니다.");
        setStreamPanel((prev) => ({ ...prev, isOpen: false }));
      }
    });
    return unsub;
  }, []);

  useEffect(() => {
    const unsub = onStream((chunk: StreamChunk) => {
      setStreamPanel((prev) => {
        const nextFile = chunk.fileName || null;
        const isNewFile = chunk.reset || (nextFile && nextFile !== prev.fileName);
        const base = isNewFile ? "" : prev.text;
        const nextText = base + (chunk.delta || "");
        return {
          fileName: nextFile,
          text: nextText,
          isOpen: true,
        };
      });
    });
    return unsub;
  }, []);

  async function refreshListAndDistributions() {
    const [rows, distAvg, distCount, vs] = await Promise.all([
      listPhotos(),
      getDistribution("avg_score"),
      getDistribution("count_ratio"),
      getValueStats().catch(() => ({ valuable: 0, notValuable: 0, unknown: 0 })),
    ]);
    setPhotos(rows);
    setDistributionAvg(distAvg);
    setDistributionCount(distCount);
    setValueStats(vs);
  }

  const [exportManuallySet, setExportManuallySet] = useState(false);

  const updateSourceRoot = (v: string) => {
    setSourceRoot(v);
    if (!exportManuallySet) {
      const base = v.replace(/\/+$/, "");
      if (base) setExportRoot(`${base}/분류됨`);
      else setExportRoot("");
    }
  };
  const updateExportRoot = (v: string) => {
    setExportManuallySet(Boolean(v.trim()));
    setExportRoot(v);
  };

  const saveSettings = (next: {
    baseUrl: string;
    model: string;
    think: boolean;
    stream: boolean;
    resizeEnabled: boolean;
    maxEdge: number;
    jpegQuality: number;
    valueEnabled: boolean;
    concurrency: number;
    engine: "clip" | "ollama";
    clipModelDir: string;
    clipModelFile: string;
    clipFallbackToOllama: boolean;
    clipEpAuto: boolean;
    clipEpCoreml: boolean;
    clipEpCuda: boolean;
    clipEpRocm: boolean;
    clipEpDirectml: boolean;
    clipEpOpenvino: boolean;
  }) => {
    const maxEdge = Math.min(4096, Math.max(128, Math.floor(next.maxEdge || 0)));
    const jpegQuality = Math.min(95, Math.max(20, Math.floor(next.jpegQuality || 0)));
    const concurrency = Math.min(32, Math.max(1, Math.floor(next.concurrency || 1)));
    const stream = concurrency > 1 ? false : next.stream;
    apiSetSettings({
      ollamaBaseUrl: next.baseUrl,
      ollamaModel: next.model,
      ollamaThink: next.think,
      ollamaStream: stream,
      analysisResizeEnabled: next.resizeEnabled,
      analysisMaxEdge: maxEdge,
      analysisJpegQuality: jpegQuality,
      analysisValueEnabled: Boolean(next.valueEnabled),
      analysisConcurrency: concurrency,
      analysisEngine: next.engine,
      clipModelDir: next.clipModelDir.trim() ? next.clipModelDir.trim() : null,
      clipModelFile: next.clipModelFile.trim() || "onnx/model_q4f16.onnx",
      clipFallbackToOllama: next.clipFallbackToOllama,
      clipEpAuto: next.clipEpAuto,
      clipEpCoreml: next.clipEpCoreml,
      clipEpCuda: next.clipEpCuda,
      clipEpRocm: next.clipEpRocm,
      clipEpDirectml: next.clipEpDirectml,
      clipEpOpenvino: next.clipEpOpenvino,
    }).then(() => {
      setSettingsBaseUrl(next.baseUrl);
      setSettingsModel(next.model);
      setSettingsThink(next.think);
      setSettingsStream(stream);
      setSettingsResizeEnabled(next.resizeEnabled);
      setSettingsMaxEdge(maxEdge);
      setSettingsJpegQuality(jpegQuality);
      setSettingsValueEnabled(Boolean(next.valueEnabled));
      setSettingsConcurrency(concurrency);
      setSettingsEngine(next.engine);
      setSettingsClipModelDir(next.clipModelDir.trim() ? next.clipModelDir.trim() : "");
      setSettingsClipModelFile(next.clipModelFile.trim() || "onnx/model_q4f16.onnx");
      setSettingsClipFallback(next.clipFallbackToOllama);
      setSettingsClipEpAuto(next.clipEpAuto);
      setSettingsClipEpCoreml(next.clipEpCoreml);
      setSettingsClipEpCuda(next.clipEpCuda);
      setSettingsClipEpRocm(next.clipEpRocm);
      setSettingsClipEpDirectml(next.clipEpDirectml);
      setSettingsClipEpOpenvino(next.clipEpOpenvino);
      getClipModelFiles()
        .then((files) => setClipModelFiles(files))
        .catch(() => setClipModelFiles([]));
      toast.success("저장되었습니다");
    });
  };

  const testConnection = async (baseUrl: string) => {
    const res = await testOllama(baseUrl);
    if (!res.ok) {
      toast.error(res.message);
      return;
    }
    toast.success(res.message);
    const models = await listOllamaModels(baseUrl);
    setAvailableModels(models);
    if (!models.length) toast.warning("모델 목록을 가져오지 못했습니다. Ollama에 모델이 있는지 확인하세요.");
  };

  const startAnalysisNow = async () => {
    if (!sourceRoot || !exportRoot) {
      toast.error("소스/Export 경로를 입력하세요");
      return;
    }
    const shouldCheckOllama =
      settingsEngine === "ollama" || (settingsEngine === "clip" && settingsClipFallback);
    if (shouldCheckOllama) {
      const conn = await testOllama(settingsBaseUrl);
      if (!conn.ok) {
        toast.error(conn.message);
        return;
      }
      if (!settingsModel) {
        toast.error("Ollama 모델을 선택/저장하세요");
        return;
      }
    }
    setStarting(true);
    try {
      jobStartedAtRef.current = Date.now();
      setJobElapsedMs(0);
      setStreamPanel({ fileName: null, text: "", isOpen: false });
      const { jobId } = await startAnalysis({ sourceRoot, exportRoot });
      setProgress({
        jobId,
        status: "running",
        processed: 0,
        total: 0,
        errors: 0,
      });
      setDistributionAvg(null);
      setDistributionCount(null);
      setValueStats(null);
      toast.info("분석을 시작했습니다");
    } catch (e) {
      toast.error("시작에 실패했습니다");
    } finally {
      setStarting(false);
    }
  };

  const cancelCurrent = async () => {
    if (!progress.jobId) return;
    await cancelAnalysis(progress.jobId);
  };

  const setCategoryFilter = (c: CategoryKey | "all") => setCategoryFilterState(c);

  const loadPhotoDetail = async (id: string) => {
    setLoadingDetail(true);
    try {
      const detail = await getPhotoDetail(id);
      setSelectedPhoto(detail);
    } catch (e) {
      toast.error("세부 정보를 불러올 수 없습니다");
    } finally {
      setLoadingDetail(false);
    }
  };

  const closeDetail = () => setSelectedPhoto(null);

  const resetResults = async () => {
    if (progress.status === "running") {
      toast.warning("진행 중인 분석을 중지한 뒤 초기화하세요.");
      return;
    }
    await apiClearResults();
    setPhotos([]);
    setDistributionAvg(null);
    setDistributionCount(null);
    setValueStats(null);
    setSelectedPhoto(null);
    setProgress({
      jobId: "",
      status: "idle",
      processed: 0,
      total: 0,
      errors: 0,
    });
    jobStartedAtRef.current = null;
    setJobElapsedMs(null);
    toast.success("결과 목록이 초기화되었습니다");
  };

  const value: AnalysisContextValue = useMemo(
    () => ({
      sourceRoot,
      exportRoot,
      settingsBaseUrl,
      settingsModel,
      settingsThink,
      settingsStream,
      settingsResizeEnabled,
      settingsMaxEdge,
      settingsJpegQuality,
      settingsValueEnabled,
      settingsConcurrency,
      settingsEngine,
      settingsClipModelDir,
      settingsClipModelFile,
      settingsClipFallback,
      settingsClipEpAuto,
      settingsClipEpCoreml,
      settingsClipEpCuda,
      settingsClipEpRocm,
      settingsClipEpDirectml,
      settingsClipEpOpenvino,
      clipAccelCaps,
      clipModelFiles,
      availableModels,
      jobElapsedMs,
      progress,
      photos,
      distributionAvg,
      distributionCount,
      valueStats,
      categoryFilter,
      selectedPhoto,
      loadingDetail,
      starting,
      streamPanel,
      updateSourceRoot,
      updateExportRoot,
      saveSettings,
      testConnection,
      startAnalysisNow,
      cancelCurrent,
      setCategoryFilter,
      loadPhotoDetail,
      closeDetail,
      resetResults,
    }),
    [
      categoryFilter,
      distributionAvg,
      distributionCount,
      valueStats,
      exportRoot,
      jobElapsedMs,
      loadingDetail,
      photos,
      progress,
      resetResults,
      selectedPhoto,
      settingsBaseUrl,
      settingsModel,
      settingsThink,
      settingsStream,
      settingsResizeEnabled,
      settingsMaxEdge,
      settingsJpegQuality,
      settingsValueEnabled,
      settingsConcurrency,
      settingsEngine,
      settingsClipModelDir,
      settingsClipModelFile,
      settingsClipFallback,
      settingsClipEpAuto,
      settingsClipEpCoreml,
      settingsClipEpCuda,
      settingsClipEpRocm,
      settingsClipEpDirectml,
      settingsClipEpOpenvino,
      clipAccelCaps,
      clipModelFiles,
      sourceRoot,
      starting,
      startAnalysisNow,
      availableModels,
      streamPanel,
    ]
  );

  return <AnalysisContext.Provider value={value}>{children}</AnalysisContext.Provider>;
}

export function useAnalysis() {
  const ctx = useContext(AnalysisContext);
  if (!ctx) throw new Error("useAnalysis must be used within AnalysisProvider");
  return ctx;
}

export { categories };
