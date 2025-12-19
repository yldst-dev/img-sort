import PrimaryButton from "../../components/buttons/PrimaryButton";

interface SettingsHomeProps {
  activeEngine: "clip" | "ollama";
  onSelectEngine: (engine: "clip" | "ollama") => void;
  onOpenOllama: () => void;
  onOpenClip: () => void;
}

function SettingsHome({
  activeEngine,
  onSelectEngine,
  onOpenOllama,
  onOpenClip,
}: SettingsHomeProps) {
  const isOllamaActive = activeEngine === "ollama";
  const isClipActive = activeEngine === "clip";

  return (
    <div className="section">
      <div className="grid two">
        <div className="section card" style={{ borderTop: "none" }}>
          <div className="flex-between" style={{ gap: 12 }}>
            <div className="section-title">Ollama 설정</div>
            <div className="toggle-group">
              <button className={isOllamaActive ? "active" : ""} onClick={() => onSelectEngine("ollama")}>
                ON
              </button>
              <button className={!isOllamaActive ? "active" : ""} onClick={() => onSelectEngine("clip")}>
                OFF
              </button>
            </div>
          </div>
          <p className="muted" style={{ marginTop: 6 }}>
            Base URL / 모델 선택 / Think / Stream 등 Ollama 관련 설정을 관리합니다.
          </p>
          <div style={{ display: "flex", gap: 10, marginTop: 12 }}>
            <PrimaryButton onClick={onOpenOllama} disabled={!isOllamaActive}>
              열기
            </PrimaryButton>
          </div>
        </div>
        <div className="section card">
          <div className="flex-between" style={{ gap: 12 }}>
            <div className="section-title">CLIP 설정</div>
            <div className="toggle-group">
              <button className={isClipActive ? "active" : ""} onClick={() => onSelectEngine("clip")}>
                ON
              </button>
              <button className={!isClipActive ? "active" : ""} onClick={() => onSelectEngine("ollama")}>
                OFF
              </button>
            </div>
          </div>
          <p className="muted" style={{ marginTop: 6 }}>
            분류 엔진 / 모델 경로 / 가속(EP) / 병렬 처리 등 CLIP 관련 설정을 관리합니다.
          </p>
          <div style={{ display: "flex", gap: 10, marginTop: 12 }}>
            <PrimaryButton onClick={onOpenClip} disabled={!isClipActive}>
              열기
            </PrimaryButton>
          </div>
        </div>
      </div>
    </div>
  );
}

export default SettingsHome;
