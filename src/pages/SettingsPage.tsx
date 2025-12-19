import { useEffect, useMemo, useState } from "react";
import PrimaryButton from "../components/buttons/PrimaryButton";
import { useAnalysis } from "../features/analysis/store";
import SettingsHome from "./settings/SettingsHome";
import OllamaSettings from "./settings/OllamaSettings";
import ClipSettings from "./settings/ClipSettings";

interface SettingsPageProps {
  theme: "light" | "dark";
  onToggleTheme: () => void;
}

type SettingsView = "home" | "ollama" | "clip";

function SettingsPage({ theme, onToggleTheme }: SettingsPageProps) {
  const {
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
    resetResults,
    saveSettings,
  } = useAnalysis();

  const [view, setView] = useState<SettingsView>("home");

  const [baseUrl, setBaseUrl] = useState(settingsBaseUrl);
  const [model, setModel] = useState(settingsModel);
  const [think, setThink] = useState(settingsThink);
  const [stream, setStream] = useState(settingsStream);
  const [resizeEnabled, setResizeEnabled] = useState(settingsResizeEnabled);
  const [maxEdge, setMaxEdge] = useState(settingsMaxEdge);
  const [jpegQuality, setJpegQuality] = useState(settingsJpegQuality);
  const [valueEnabled, setValueEnabled] = useState(settingsValueEnabled);
  const [concurrency, setConcurrency] = useState(settingsConcurrency);
  const [engine, setEngine] = useState<"clip" | "ollama">(settingsEngine);
  const [clipModelDir, setClipModelDir] = useState(settingsClipModelDir);
  const [clipModelFile, setClipModelFile] = useState(settingsClipModelFile);
  const [clipFallbackToOllama, setClipFallbackToOllama] =
    useState(settingsClipFallback);
  const [clipEpAuto, setClipEpAuto] = useState(settingsClipEpAuto);
  const [clipEpCoreml, setClipEpCoreml] = useState(settingsClipEpCoreml);
  const [clipEpCuda, setClipEpCuda] = useState(settingsClipEpCuda);
  const [clipEpRocm, setClipEpRocm] = useState(settingsClipEpRocm);
  const [clipEpDirectml, setClipEpDirectml] = useState(settingsClipEpDirectml);
  const [clipEpOpenvino, setClipEpOpenvino] = useState(settingsClipEpOpenvino);

  useEffect(() => setBaseUrl(settingsBaseUrl), [settingsBaseUrl]);
  useEffect(() => setModel(settingsModel), [settingsModel]);
  useEffect(() => setThink(settingsThink), [settingsThink]);
  useEffect(() => setStream(settingsStream), [settingsStream]);
  useEffect(() => setResizeEnabled(settingsResizeEnabled), [settingsResizeEnabled]);
  useEffect(() => setMaxEdge(settingsMaxEdge), [settingsMaxEdge]);
  useEffect(() => setJpegQuality(settingsJpegQuality), [settingsJpegQuality]);
  useEffect(() => setValueEnabled(settingsValueEnabled), [settingsValueEnabled]);
  useEffect(() => setConcurrency(settingsConcurrency), [settingsConcurrency]);
  useEffect(() => setEngine(settingsEngine), [settingsEngine]);
  useEffect(() => setClipModelDir(settingsClipModelDir), [settingsClipModelDir]);
  useEffect(() => setClipModelFile(settingsClipModelFile), [settingsClipModelFile]);
  useEffect(() => setClipFallbackToOllama(settingsClipFallback), [settingsClipFallback]);
  useEffect(() => setClipEpAuto(settingsClipEpAuto), [settingsClipEpAuto]);
  useEffect(() => setClipEpCoreml(settingsClipEpCoreml), [settingsClipEpCoreml]);
  useEffect(() => setClipEpCuda(settingsClipEpCuda), [settingsClipEpCuda]);
  useEffect(() => setClipEpRocm(settingsClipEpRocm), [settingsClipEpRocm]);
  useEffect(() => setClipEpDirectml(settingsClipEpDirectml), [settingsClipEpDirectml]);
  useEffect(() => setClipEpOpenvino(settingsClipEpOpenvino), [settingsClipEpOpenvino]);

  const onSaveAll = () =>
    saveSettings({
      baseUrl,
      model,
      think,
      stream,
      resizeEnabled,
      maxEdge,
      jpegQuality,
      valueEnabled,
      concurrency,
      engine,
      clipModelDir,
      clipModelFile,
      clipFallbackToOllama,
      clipEpAuto,
      clipEpCoreml,
      clipEpCuda,
      clipEpRocm,
      clipEpDirectml,
      clipEpOpenvino,
    });

  const ollamaDraft = useMemo(
    () => ({
      baseUrl,
      model,
      think,
      stream,
      resizeEnabled,
      maxEdge,
      jpegQuality,
      concurrency,
    }),
    [baseUrl, model, think, stream, resizeEnabled, maxEdge, jpegQuality, concurrency]
  );

  const clipDraft = useMemo(
    () => ({
      clipModelDir,
      clipModelFile,
      clipFallbackToOllama,
      clipEpAuto,
      clipEpCoreml,
      clipEpCuda,
      clipEpRocm,
      clipEpDirectml,
      clipEpOpenvino,
      valueEnabled,
      concurrency,
    }),
    [
      clipEpAuto,
      clipEpCoreml,
      clipEpCuda,
      clipEpDirectml,
      clipEpOpenvino,
      clipEpRocm,
      clipFallbackToOllama,
      clipModelDir,
      clipModelFile,
      valueEnabled,
      concurrency,
    ]
  );

  const title = view === "home" ? "설정" : view === "ollama" ? "Ollama 설정" : "CLIP 설정";

  return (
    <div className="page">
      <div className="flex-between">
        <div>
          <h1>{title}</h1>
          <p className="muted">
            {view === "home"
              ? "원하는 설정 섹션으로 이동하세요"
              : "수정 후 Save를 눌러 저장하세요"}
          </p>
        </div>
        <div style={{ textAlign: "right" }}>
          <div className="muted" style={{ fontWeight: 700, marginBottom: 6 }}>
            테마
          </div>
          <div className="toggle-group">
            <button
              className={theme === "light" ? "active" : ""}
              onClick={() => theme !== "light" && onToggleTheme()}
            >
              라이트
            </button>
            <button
              className={theme === "dark" ? "active" : ""}
              onClick={() => theme !== "dark" && onToggleTheme()}
            >
              다크
            </button>
          </div>
        </div>
      </div>

      {view !== "home" && (
        <div style={{ display: "flex", gap: 10, marginTop: 12 }}>
          <PrimaryButton variant="secondary" onClick={() => setView("home")}>
            ← 설정 홈
          </PrimaryButton>
        </div>
      )}

      {view === "home" ? (
        <>
          <SettingsHome
            activeEngine={engine}
            onSelectEngine={(next) => {
              setEngine(next);
              // Engine 전환은 즉시 저장해 UX를 단순화합니다.
              saveSettings({
                baseUrl,
                model,
                think,
                stream,
                resizeEnabled,
                maxEdge,
                jpegQuality,
                valueEnabled,
                concurrency,
                engine: next,
                clipModelDir,
                clipModelFile,
                clipFallbackToOllama,
                clipEpAuto,
                clipEpCoreml,
                clipEpCuda,
                clipEpRocm,
                clipEpDirectml,
                clipEpOpenvino,
              });
            }}
            onOpenOllama={() => setView("ollama")}
            onOpenClip={() => setView("clip")}
          />

          <div className="section card" style={{ borderTop: "none" }}>
            <div className="section-title">결과 초기화</div>
            <p className="muted" style={{ marginTop: 6 }}>
              결과 테이블의 과거 목록(분석 결과 DB)을 모두 삭제합니다.
            </p>
            <div style={{ display: "flex", gap: 10, marginTop: 12 }}>
              <PrimaryButton
                variant="danger"
                onClick={async () => {
                  const ok = window.confirm("결과 목록을 모두 삭제할까요? (되돌릴 수 없습니다)");
                  if (!ok) return;
                  await resetResults();
                }}
              >
                초기화
              </PrimaryButton>
            </div>
          </div>
        </>
      ) : view === "ollama" ? (
        <OllamaSettings
          disabled={engine !== "ollama"}
          draft={ollamaDraft}
          onChange={(next) => {
            if (next.baseUrl !== undefined) setBaseUrl(next.baseUrl);
            if (next.model !== undefined) setModel(next.model);
            if (next.think !== undefined) setThink(next.think);
            if (next.stream !== undefined) setStream(next.stream);
            if (next.resizeEnabled !== undefined) setResizeEnabled(next.resizeEnabled);
            if (next.maxEdge !== undefined) setMaxEdge(next.maxEdge);
            if (next.jpegQuality !== undefined) setJpegQuality(next.jpegQuality);
            if (next.concurrency !== undefined) setConcurrency(next.concurrency);
          }}
          onSave={onSaveAll}
        />
      ) : (
        <ClipSettings
          disabled={engine !== "clip"}
          draft={clipDraft}
          onChange={(next) => {
            if (next.clipModelDir !== undefined) setClipModelDir(next.clipModelDir);
            if (next.clipModelFile !== undefined) setClipModelFile(next.clipModelFile);
            if (next.clipFallbackToOllama !== undefined)
              setClipFallbackToOllama(next.clipFallbackToOllama);
            if (next.clipEpAuto !== undefined) setClipEpAuto(next.clipEpAuto);
            if (next.clipEpCoreml !== undefined) setClipEpCoreml(next.clipEpCoreml);
            if (next.clipEpCuda !== undefined) setClipEpCuda(next.clipEpCuda);
            if (next.clipEpRocm !== undefined) setClipEpRocm(next.clipEpRocm);
            if (next.clipEpDirectml !== undefined) setClipEpDirectml(next.clipEpDirectml);
            if (next.clipEpOpenvino !== undefined) setClipEpOpenvino(next.clipEpOpenvino);
            if (next.valueEnabled !== undefined) setValueEnabled(next.valueEnabled);
            if (next.concurrency !== undefined) setConcurrency(next.concurrency);
          }}
          onSave={onSaveAll}
        />
      )}
    </div>
  );
}

export default SettingsPage;
