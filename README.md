# Tauri + React + Typescript

This template should help get you started developing with Tauri, React and Typescript in Vite.

## CLIP 모델 위치

- 기본 CLIP ONNX 모델 디렉터리: `models/clip-vit-b32-onnx/`
- 빌드 시 Tauri 번들 리소스로 포함되며, 개발 모드에서는 프로젝트 디렉터리에서 직접 읽습니다.
- CLIP 기본 모델 파일은 `models/clip-vit-b32-onnx/onnx/model_q4f16.onnx` 입니다.

## CLIP 가속(Execution Providers)

- 설정에서 EP를 ON/OFF 할 수 있습니다. Auto ON이면 켠 후보 중 사용 가능한 EP를 우선순위로 시도하고, 실패 시 CPU로 폴백합니다.
- 현재 macOS 빌드에서는 `ort`의 `coreml` 기능을 포함해 CoreML EP 감지가 가능합니다.

## 저장 가치 판단(1단계 분류)

분석 옵션에서 **“저장 가치 판단(1단계)”**를 ON으로 켜면, 카테고리 분류(8개) 전에 **이 이미지가 저장할 가치가 있는지**를 먼저 판단합니다.

### 판단 방식(조건)

- 엔진: **CLIP(clip-vit-b32-onnx)**
- 방식: 이미지 임베딩과 텍스트 임베딩 간 **cosine similarity**를 사용합니다.
- 텍스트 프롬프트(요약):
  - **Keep(가치 있음)**: “worth keeping”, “meaningful”, “high quality”, “important screenshot” 류
  - **Drop(가치 없음)**: “not worth keeping”, “blurry/accidental”, “duplicate/unimportant” 류
- 계산:
  1) 이미지 임베딩을 구합니다(vision).
  2) keep/drop 텍스트 임베딩(앱 시작 시 캐시)과 각각 유사도를 계산합니다.
  3) `[keep_logit, drop_logit]`에 softmax를 적용해 `keep_prob`를 구합니다.
  4) `keep_prob >= 0.5`이면 **가치 있음**, 아니면 **가치 없음**으로 저장합니다.

### 결과 표시

- 결과 페이지에서 **전체 분포 레이더 아래**에 “가치 있음/없음” 가로 막대 그래프가 추가로 표시됩니다.
- 이 기능은 현재 **CLIP 엔진에서만** 동작하며, Ollama 엔진은 가치 판단을 저장하지 않습니다.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
