import { useEffect, useState } from "react";
import TextField from "../../components/forms/TextField";
import PrimaryButton from "../../components/buttons/PrimaryButton";
import { useAnalysis } from "../../features/analysis/store";

interface OllamaSettingsProps {
  disabled?: boolean;
  draft: {
    baseUrl: string;
    model: string;
    think: boolean;
    stream: boolean;
    resizeEnabled: boolean;
    maxEdge: number;
    jpegQuality: number;
    concurrency: number;
  };
  onChange: (next: Partial<OllamaSettingsProps["draft"]>) => void;
  onSave: () => void;
}

function OllamaSettings({ disabled = false, draft, onChange, onSave }: OllamaSettingsProps) {
  const { availableModels, testConnection } = useAnalysis();
  const [testing, setTesting] = useState(false);
  const streamLockedByConcurrency = (draft.concurrency || 1) > 1;

  useEffect(() => {
    if (!availableModels.length) return;
    if (!draft.model || !availableModels.includes(draft.model)) {
      onChange({ model: availableModels[0] });
    }
  }, [availableModels, draft.model, onChange]);

  useEffect(() => {
    if (streamLockedByConcurrency && draft.stream) onChange({ stream: false });
  }, [draft.stream, streamLockedByConcurrency, onChange]);

  return (
    <div
      className="section"
      style={{
        opacity: disabled ? 0.55 : 1,
        pointerEvents: disabled ? "none" : "auto",
      }}
    >
      <div className="grid two">
        <TextField
          label="Ollama Base URL"
          value={draft.baseUrl}
          onChange={(e) => onChange({ baseUrl: e.target.value })}
          placeholder="http://127.0.0.1:11434"
          fullWidth
        />
      </div>

      <div className="section" style={{ marginTop: 12 }}>
        <div className="textfield">
          <label className="muted" htmlFor="ollama-model">
            Ollama Model
          </label>
          <select
            id="ollama-model"
            className="select"
            value={draft.model}
            onChange={(e) => onChange({ model: e.target.value })}
            disabled={!availableModels.length}
          >
            {availableModels.length ? (
              availableModels.map((m) => (
                <option key={m} value={m}>
                  {m}
                </option>
              ))
            ) : (
              <option value={draft.model || ""}>Test Connection 후 모델 목록이 표시됩니다</option>
            )}
          </select>
        </div>
      </div>

      <div style={{ display: "flex", gap: 10, marginTop: 12 }}>
        <PrimaryButton
          variant="secondary"
          loading={testing}
          onClick={async () => {
            setTesting(true);
            await testConnection(draft.baseUrl);
            setTesting(false);
          }}
        >
          Test Connection
        </PrimaryButton>
      </div>

      <div className="section card" style={{ marginTop: 12 }}>
        <div className="section-title">Reasoning(Think)</div>
        <p className="muted" style={{ marginTop: 6 }}>
          OFF로 두면 모델의 reasoning 출력을 줄여 JSON 응답 안정성이 올라갈 수 있습니다.
        </p>
        <div className="toggle-group" style={{ marginTop: 10 }}>
          <button className={!draft.think ? "active" : ""} onClick={() => onChange({ think: false })}>
            OFF
          </button>
          <button className={draft.think ? "active" : ""} onClick={() => onChange({ think: true })}>
            ON
          </button>
        </div>
      </div>

      <div className="section card" style={{ marginTop: 12 }}>
        <div className="section-title">Stream</div>
        <p className="muted" style={{ marginTop: 6 }}>
          Ollama 응답을 스트리밍으로 받아 현재 생성 중인 내용을 실시간으로 표시합니다.
        </p>
        {streamLockedByConcurrency && (
          <p className="muted" style={{ marginTop: 6, fontSize: 12 }}>
            병렬 처리(동시 처리 수 2 이상)에서는 출력이 섞이기 때문에 Stream은 자동으로 OFF로 고정됩니다.
          </p>
        )}
        <div className="toggle-group" style={{ marginTop: 10 }}>
          <button
            className={!draft.stream ? "active" : ""}
            onClick={() => onChange({ stream: false })}
            disabled={streamLockedByConcurrency}
          >
            OFF
          </button>
          <button
            className={draft.stream ? "active" : ""}
            onClick={() => onChange({ stream: true })}
            disabled={streamLockedByConcurrency}
          >
            ON
          </button>
        </div>
      </div>

      <div className="section card" style={{ marginTop: 12 }}>
        <div className="section-title">분석 속도(이미지 최적화)</div>
        <p className="muted" style={{ marginTop: 6 }}>
          Ollama로 보내는 이미지를 축소/압축해서 더 빠르게 분석합니다. (권장)
        </p>
        <div className="toggle-group" style={{ marginTop: 10 }}>
          <button
            className={draft.resizeEnabled ? "active" : ""}
            onClick={() => onChange({ resizeEnabled: true })}
          >
            ON
          </button>
          <button
            className={!draft.resizeEnabled ? "active" : ""}
            onClick={() => onChange({ resizeEnabled: false })}
          >
            OFF
          </button>
        </div>
        <div className="grid two" style={{ marginTop: 12 }}>
          <TextField
            label="분석용 최대 변(px)"
            type="number"
            inputMode="numeric"
            value={String(draft.maxEdge)}
            min={128}
            max={4096}
            step={64}
            onChange={(e) => onChange({ maxEdge: Number(e.target.value) })}
            helperText="예: 512~1024 권장 (작을수록 빠름)"
            fullWidth
            disabled={!draft.resizeEnabled}
          />
          <TextField
            label="JPEG 품질(20~95)"
            type="number"
            inputMode="numeric"
            value={String(draft.jpegQuality)}
            min={20}
            max={95}
            step={5}
            onChange={(e) => onChange({ jpegQuality: Number(e.target.value) })}
            helperText="예: 50~70 권장 (낮을수록 빠름/작음)"
            fullWidth
            disabled={!draft.resizeEnabled}
          />
        </div>
      </div>

      <div style={{ display: "flex", justifyContent: "flex-end", marginTop: 18 }}>
        <PrimaryButton onClick={onSave} disabled={disabled}>
          Save
        </PrimaryButton>
      </div>
    </div>
  );
}

export default OllamaSettings;
