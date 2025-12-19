import TextField from "../../components/forms/TextField";
import PrimaryButton from "../../components/buttons/PrimaryButton";
import { useAnalysis } from "../../features/analysis/store";

interface ClipSettingsProps {
  disabled?: boolean;
  draft: {
    clipModelDir: string;
    clipModelFile: string;
    clipFallbackToOllama: boolean;
    clipEpAuto: boolean;
    clipEpCoreml: boolean;
    clipEpCuda: boolean;
    clipEpRocm: boolean;
    clipEpDirectml: boolean;
    clipEpOpenvino: boolean;
    valueEnabled: boolean;
    concurrency: number;
  };
  onChange: (next: Partial<ClipSettingsProps["draft"]>) => void;
  onSave: () => void;
}

function ClipSettings({ disabled = false, draft, onChange, onSave }: ClipSettingsProps) {
  const { clipAccelCaps, clipModelFiles } = useAnalysis();
  const files = clipModelFiles.length ? clipModelFiles : [draft.clipModelFile].filter(Boolean);
  const hasCurrent = files.includes(draft.clipModelFile);
  const options = hasCurrent ? files : [draft.clipModelFile, ...files];

  return (
    <div
      className="section"
      style={{
        opacity: disabled ? 0.55 : 1,
        pointerEvents: disabled ? "none" : "auto",
      }}
    >
      <div className="section card">
        <div className="section-title">CLIP 모델</div>
        <p className="muted" style={{ marginTop: 6 }}>
          모델 디렉터리를 지정하지 않으면 models/clip-vit-b32-onnx를 자동 탐색합니다.
        </p>
        <div className="grid" style={{ marginTop: 12 }}>
          <TextField
            label="CLIP 모델 디렉터리(선택)"
            value={draft.clipModelDir}
            onChange={(e) => onChange({ clipModelDir: e.target.value })}
            placeholder="비워두면 models/clip-vit-b32-onnx 자동 탐색"
            fullWidth
          />
          <div className="textfield">
            <label className="muted" htmlFor="clip-model-file">
              ONNX 모델 파일
            </label>
            <select
              id="clip-model-file"
              className="select"
              value={draft.clipModelFile}
              onChange={(e) => onChange({ clipModelFile: e.target.value })}
            >
              {options.map((f) => (
                <option key={f} value={f}>
                  {f}
                </option>
              ))}
            </select>
            {!hasCurrent && (
              <div className="muted" style={{ marginTop: 6, fontSize: 12 }}>
                현재 선택된 파일이 디렉터리에서 발견되지 않았습니다. 저장 후 경로/번들 포함 여부를 확인하세요.
              </div>
            )}
          </div>
        </div>
        <div className="section" style={{ marginTop: 12 }}>
          <div className="section-title" style={{ fontSize: 13 }}>
            CLIP 실패 시 Ollama 폴백
          </div>
          <div className="toggle-group" style={{ marginTop: 10 }}>
            <button
              className={draft.clipFallbackToOllama ? "active" : ""}
              onClick={() => onChange({ clipFallbackToOllama: true })}
            >
              ON
            </button>
            <button
              className={!draft.clipFallbackToOllama ? "active" : ""}
              onClick={() => onChange({ clipFallbackToOllama: false })}
            >
              OFF
            </button>
          </div>
        </div>
      </div>

      <div className="section card" style={{ marginTop: 12 }}>
        <div className="section-title">저장 가치 판단(1단계)</div>
        <p className="muted" style={{ marginTop: 6 }}>
          사진/스크린샷이 “저장할 가치가 있는지”를 먼저 판단합니다. ON이면 결과 페이지에서 가치 있음/없음
          개수 막대가 추가로 표시됩니다.
        </p>
        <div className="toggle-group" style={{ marginTop: 10 }}>
          <button
            className={draft.valueEnabled ? "active" : ""}
            onClick={() => onChange({ valueEnabled: true })}
          >
            ON
          </button>
          <button
            className={!draft.valueEnabled ? "active" : ""}
            onClick={() => onChange({ valueEnabled: false })}
          >
            OFF
          </button>
        </div>
      </div>

      <div className="section card" style={{ marginTop: 12 }}>
        <div className="section-title">병렬 처리(멀티스레드)</div>
        <p className="muted" style={{ marginTop: 6 }}>
          동시에 처리할 이미지 개수입니다. 값이 클수록 빨라질 수 있지만 CPU/GPU/메모리 사용량이 증가합니다.
          Stream ON일 때는 출력이 섞이는 문제를 방지하기 위해 내부적으로 1로 동작합니다.
        </p>
        <div className="grid two" style={{ marginTop: 12 }}>
          <TextField
            label="동시 처리 수"
            type="number"
            inputMode="numeric"
            value={String(draft.concurrency)}
            min={1}
            max={32}
            step={1}
            onChange={(e) => onChange({ concurrency: Number(e.target.value) })}
            helperText="권장: 2~8 (코어 수에 따라 조절)"
            fullWidth
          />
        </div>
      </div>

      <div className="section card" style={{ marginTop: 12 }}>
        <div className="section-title">CLIP 가속(Execution Providers)</div>
        <p className="muted" style={{ marginTop: 6 }}>
          Auto ON이면 아래에서 켠(ON) 가속 후보 중 사용 가능한 EP를 우선순위로 시도합니다.
        </p>
        <div className="toggle-group" style={{ marginTop: 10 }}>
          <button className={draft.clipEpAuto ? "active" : ""} onClick={() => onChange({ clipEpAuto: true })}>
            Auto ON
          </button>
          <button className={!draft.clipEpAuto ? "active" : ""} onClick={() => onChange({ clipEpAuto: false })}>
            Auto OFF(CPU 고정)
          </button>
        </div>

        <div className="grid two" style={{ marginTop: 12 }}>
          <div className="section card" style={{ border: "none", padding: 0 }}>
            <div className="section-title" style={{ fontSize: 13 }}>
              Apple CoreML (M칩/ANE/GPU)
            </div>
            <div className="toggle-group" style={{ marginTop: 10 }}>
              <button
                className={draft.clipEpCoreml ? "active" : ""}
                onClick={() => onChange({ clipEpCoreml: true })}
                disabled={clipAccelCaps ? !clipAccelCaps.coreml.available : false}
              >
                ON
              </button>
              <button
                className={!draft.clipEpCoreml ? "active" : ""}
                onClick={() => onChange({ clipEpCoreml: false })}
                disabled={clipAccelCaps ? !clipAccelCaps.coreml.supported : false}
              >
                OFF
              </button>
            </div>
            {clipAccelCaps && !clipAccelCaps.coreml.available && (
              <p className="muted" style={{ marginTop: 6, fontSize: 12 }}>
                사용 불가: {clipAccelCaps.coreml.name}
              </p>
            )}
          </div>

          <div className="section card" style={{ border: "none", padding: 0 }}>
            <div className="section-title" style={{ fontSize: 13 }}>
              NVIDIA CUDA
            </div>
            <div className="toggle-group" style={{ marginTop: 10 }}>
              <button
                className={draft.clipEpCuda ? "active" : ""}
                onClick={() => onChange({ clipEpCuda: true })}
                disabled={clipAccelCaps ? !clipAccelCaps.cuda.available : false}
              >
                ON
              </button>
              <button
                className={!draft.clipEpCuda ? "active" : ""}
                onClick={() => onChange({ clipEpCuda: false })}
                disabled={clipAccelCaps ? !clipAccelCaps.cuda.supported : false}
              >
                OFF
              </button>
            </div>
            {clipAccelCaps && !clipAccelCaps.cuda.available && (
              <p className="muted" style={{ marginTop: 6, fontSize: 12 }}>
                사용 불가: {clipAccelCaps.cuda.name}
              </p>
            )}
          </div>

          <div className="section card" style={{ border: "none", padding: 0 }}>
            <div className="section-title" style={{ fontSize: 13 }}>
              AMD ROCm
            </div>
            <div className="toggle-group" style={{ marginTop: 10 }}>
              <button
                className={draft.clipEpRocm ? "active" : ""}
                onClick={() => onChange({ clipEpRocm: true })}
                disabled={clipAccelCaps ? !clipAccelCaps.rocm.available : false}
              >
                ON
              </button>
              <button
                className={!draft.clipEpRocm ? "active" : ""}
                onClick={() => onChange({ clipEpRocm: false })}
                disabled={clipAccelCaps ? !clipAccelCaps.rocm.supported : false}
              >
                OFF
              </button>
            </div>
            {clipAccelCaps && !clipAccelCaps.rocm.available && (
              <p className="muted" style={{ marginTop: 6, fontSize: 12 }}>
                사용 불가: {clipAccelCaps.rocm.name}
              </p>
            )}
          </div>

          <div className="section card" style={{ border: "none", padding: 0 }}>
            <div className="section-title" style={{ fontSize: 13 }}>
              Windows DirectML
            </div>
            <div className="toggle-group" style={{ marginTop: 10 }}>
              <button
                className={draft.clipEpDirectml ? "active" : ""}
                onClick={() => onChange({ clipEpDirectml: true })}
                disabled={clipAccelCaps ? !clipAccelCaps.directml.available : false}
              >
                ON
              </button>
              <button
                className={!draft.clipEpDirectml ? "active" : ""}
                onClick={() => onChange({ clipEpDirectml: false })}
                disabled={clipAccelCaps ? !clipAccelCaps.directml.supported : false}
              >
                OFF
              </button>
            </div>
            {clipAccelCaps && !clipAccelCaps.directml.available && (
              <p className="muted" style={{ marginTop: 6, fontSize: 12 }}>
                사용 불가: {clipAccelCaps.directml.name}
              </p>
            )}
          </div>

          <div className="section card" style={{ border: "none", padding: 0 }}>
            <div className="section-title" style={{ fontSize: 13 }}>
              Intel OpenVINO
            </div>
            <div className="toggle-group" style={{ marginTop: 10 }}>
              <button
                className={draft.clipEpOpenvino ? "active" : ""}
                onClick={() => onChange({ clipEpOpenvino: true })}
                disabled={clipAccelCaps ? !clipAccelCaps.openvino.available : false}
              >
                ON
              </button>
              <button
                className={!draft.clipEpOpenvino ? "active" : ""}
                onClick={() => onChange({ clipEpOpenvino: false })}
                disabled={clipAccelCaps ? !clipAccelCaps.openvino.supported : false}
              >
                OFF
              </button>
            </div>
            {clipAccelCaps && !clipAccelCaps.openvino.available && (
              <p className="muted" style={{ marginTop: 6, fontSize: 12 }}>
                사용 불가: {clipAccelCaps.openvino.name}
              </p>
            )}
          </div>
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

export default ClipSettings;
