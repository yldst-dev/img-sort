import { useEffect, useState } from "react";
import TextField from "../components/forms/TextField";
import PrimaryButton from "../components/buttons/PrimaryButton";
import ProgressBar from "../components/status/ProgressBar";
import { useAnalysis } from "../features/analysis/store";
import { toast } from "../components/toast/toastBus";
import { platform } from "../lib/platform";
import { pickDirectory } from "../lib/pickDirectory";

function MainPage() {
  const {
    sourceRoot,
    exportRoot,
    jobElapsedMs,
    updateSourceRoot,
    updateExportRoot,
    startAnalysisNow,
    cancelCurrent,
    progress,
    starting,
    settingsStream,
    settingsEngine,
    settingsClipFallback,
    streamPanel,
  } = useAnalysis();

  const isRunning = progress.status === "running";
  const canStart = Boolean(sourceRoot.trim()) && Boolean(exportRoot.trim());
  const processedPct = progress.total ? progress.processed / progress.total : 0;

  const [detailLoadingId, setDetailLoadingId] = useState<string | null>(null);
  useEffect(() => {
    if (detailLoadingId) setDetailLoadingId(null);
  }, [detailLoadingId]);

  const pickFolder = async (setter: (v: string) => void, current: string) => {
    try {
      if (platform.isMac) {
        try {
          const { askForPermissions } = await import("../lib/permissions");
          await askForPermissions();
        } catch {
          // best effort; continue to picker
        }
      }

      const selected = await pickDirectory(current);
      if (selected) {
        setter(selected);
        toast.info("폴더가 선택되었습니다");
        return;
      }
    } catch (e) {
      // fall back to prompt
    }
    const value = window.prompt("경로를 입력하거나 붙여넣으세요", current);
    if (value) setter(value);
  };

  return (
    <div className="page">
      <h1>이미지 분류</h1>
      <p className="muted">소스와 Export 폴더를 지정하고 분석을 시작하세요.</p>

      <div className="section grid two">
        <div className="field-row">
          <TextField
            label="소스 폴더"
            value={sourceRoot}
            onChange={(e) => updateSourceRoot(e.target.value)}
            placeholder="/path/to/source"
            fullWidth
          />
          <PrimaryButton
            variant="secondary"
            onClick={() => pickFolder(updateSourceRoot, sourceRoot)}
            style={{ whiteSpace: "nowrap" }}
          >
            폴더 선택
          </PrimaryButton>
        </div>
        <div className="field-row">
          <TextField
            label="Export 폴더"
            value={exportRoot}
            onChange={(e) => updateExportRoot(e.target.value)}
            placeholder="/path/to/export"
            fullWidth
          />
          <PrimaryButton
            variant="secondary"
            onClick={() => pickFolder(updateExportRoot, exportRoot)}
            style={{ whiteSpace: "nowrap" }}
          >
            폴더 선택
          </PrimaryButton>
        </div>
      </div>

      <div className="section flex-between">
        <div style={{ display: "flex", gap: 8 }}>
          <PrimaryButton onClick={startAnalysisNow} loading={starting} disabled={isRunning || !canStart}>
            분석 시작
          </PrimaryButton>
          <PrimaryButton variant="ghost" onClick={cancelCurrent} disabled={!isRunning}>
            중지
          </PrimaryButton>
        </div>
        <div className="pill">상태: {progress.status}</div>
      </div>

      <div
        className={`section card progress-panel ${
          progress.status === "running" || progress.status === "completed" ? "visible" : ""
        }`}
      >
        <ProgressBar
          value={processedPct}
          status={`처리 ${progress.processed}/${progress.total} | 에러 ${progress.errors}`}
        />
        {progress.status === "completed" && typeof jobElapsedMs === "number" && (
          <div className="muted" style={{ marginTop: 8 }}>
            총 소요 시간: {(jobElapsedMs / 1000).toFixed(1)}초
          </div>
        )}
        {progress.currentFile && (
          <div className="muted" style={{ marginTop: 8 }}>
            현재 파일: {progress.currentFile}
          </div>
        )}
      </div>

      {isRunning &&
        settingsStream &&
        (settingsEngine === "ollama" || settingsClipFallback) &&
        streamPanel.isOpen && (
        <div className="section card stream-panel">
          <div className="flex-between">
            <div className="section-title">Stream</div>
            <div className="muted" style={{ fontSize: 12 }}>
              {streamPanel.fileName ? `파일: ${streamPanel.fileName}` : "대기 중…"}
            </div>
          </div>
          <pre className="stream-pre">{streamPanel.text || "…"}</pre>
        </div>
      )}
    </div>
  );
}

export default MainPage;
