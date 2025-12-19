# Repository Guidelines

## Project Structure & Module Organization
- `src/`: React + TypeScript UI (entry `main.tsx`, root layout `App.tsx`, styles in `App.css`, shared assets in `src/assets/`).
- `public/`: static files served as-is by Vite (favicons, manifest, additional images).
- `src-tauri/`: Tauri Rust backend (`main.rs` launches, commands in `lib.rs`), config in `tauri.conf.json`, icons under `icons/`.
- `index.html` and `vite.config.ts`: Vite entry/config. Keep new frontend utilities inside `src/` to benefit from bundling and TS type checks.

## Build, Test, and Development Commands
- `npm run dev`: start Vite dev server for the React UI (hot reload).
- `npm run tauri dev`: run full Tauri desktop app with Rust backend + UI.
- `npm run build`: type-check via `tsc`, then produce optimized web build.
- `npm run preview`: serve the built UI locally for smoke checks.
- `npm run tauri build`: create production desktop binaries using the Tauri CLI (requires Rust toolchain).

## Coding Style & Naming Conventions
- TypeScript is strict (see `tsconfig.json`); fix type errors instead of suppressing.
- Use 2-space indentation, single quotes/ES modules as in existing files; keep React components as function components with hooks.
- Component files: `PascalCase.tsx`; helper modules: `camelCase.ts`; CSS alongside components when small, otherwise group styles per feature.
- Place shared assets in `src/assets/`; avoid absolute filesystem paths—use Vite import paths.

## Testing Guidelines
- No automated tests yet; prefer Vitest + React Testing Library for UI and `cargo test` for Rust commands.
- Co-locate UI tests under `src/**/__tests__` with filename suffix `.test.tsx`; keep Rust tests near the code (e.g., `lib.rs` module tests).
- Aim to cover happy path + one edge case per new feature; ensure greet/command handlers are exercised before merging.

## Commit & Pull Request Guidelines
- Repository lacks Git history; adopt Conventional Commits (`feat:`, `fix:`, `chore:`, `refactor:`) for clarity and changelog generation.
- PRs should include: purpose/impact summary, linked issue (if any), test notes (`npm run dev`/`npm run tauri dev` smoke), and UI screenshots/gifs when UX changes.
- Keep diffs focused; prefer small, reviewable PRs. Coordinate breaking changes in the description and note any migration steps.

## Security & Configuration Tips
- Do not commit secrets; prefer `.env` loaded by Vite/Tauri. Add new env keys to `.gitignore` and document expected values in `README.md`.
- When adding new Tauri commands, validate inputs and keep file-system access scoped via `capabilities/` and `tauri.conf.json` to avoid privilege creep.

---

# 최종 정리(현재 구현 상태)

## 핵심 기능
- 기본 분류 엔진: **CLIP ViT-B/32 ONNX** (Ollama는 옵션으로 유지)
- export는 **복사(copy)**이며, 디렉터리 이름은 **한글**로 생성
- 카테고리 키(8개 고정):
  - `screenshot_document`, `people`, `food_cafe`, `nature_landscape`, `city_street_travel`, `pets_animals`, `products_objects`, `other`

## CLIP(ONNX) 통합
- 모델 디렉터리: `models/clip-vit-b32-onnx/` (Tauri 번들 리소스로 포함)
- Rust 모듈: `src-tauri/src/core/clip/` (engine/preprocess/math/prompts)
- 텍스트 임베딩은 앱 시작 시 캐시, 이미지마다 vision → cosine → softmax로 scores 생성
- ONNX 모델 파일은 설정에서 드롭다운으로 선택 가능(예: `onnx/model.onnx`, `onnx/model_q4f16.onnx` 등)
- 성능 로그에 로딩/추론 시간 기록

## 가속(Execution Providers) + 안정성
- Settings에서 EP ON/OFF 가능(Apple CoreML, CUDA, ROCm, DirectML, OpenVINO + Auto)
- EP 지원/가용 여부를 감지해 UI에서 비활성 처리
- CoreML 런타임 실패 케이스 대응:
  - CoreML 모델 포맷을 MLProgram + static input shapes로 설정
  - warmup에서 text/vision smoke test 후 실패하면 CoreML을 끄고 CPU로 재시도(폴백)

## 병렬 처리(멀티스레드)
- 분석 동시 처리 수(concurrency) 설정 추가
- 파이프라인을 동시 처리로 변경(JoinSet 기반)
- CLIP은 세션 풀(session pool)로 병렬 추론 지원
- Stream은 동시 처리 2 이상일 때 자동 OFF(섞임 방지)

## 1단계 “저장 가치” 판단 옵션
- CLIP 설정에 “저장 가치 판단(1단계)” ON/OFF 추가
- ON 시 keep/drop 프롬프트 기반 `keep_prob` 계산 → `is_valuable` 저장
- 결과 페이지: 가치 있음/없음 개수 막대 그래프 표시
- export 폴더 구조(가치 판단 ON일 때):
  - `분류됨/가치있음/<카테고리>/...`
  - `분류됨/가치없음/<카테고리>/...`
- CLIP 전체 분포 레이더는 export 폴더(분류 디렉터리) 폴더별 파일 개수 기반으로 계산(새 구조도 합산)

## Ollama(VLM) 관련
- Test Connection 성공 시 모델 목록을 가져와 드롭다운에 표시
- Stream ON/OFF, Reasoning(Think) 토글 지원
- 분석 시작 시 필요한 경우에만 Ollama 연결 체크 후 진행

## UI/UX 개선
- 설정 페이지를 “설정 홈 → Ollama 설정 / CLIP 설정” 하위 페이지로 분리
- 설정 홈에서 엔진을 ON/OFF 토글로 선택(한쪽만 활성), 비활성 엔진 설정은 조용히 비활성(반투명/클릭 불가)
- 토스트(우측 하단) 디자인 단순화 + 등장/퇴장 애니메이션 추가(라이트/다크 테마 대응)
- 결과 테이블:
  - 파일명 클릭 시 로그 패널에 해당 작업 로그 표시
  - 이미지별 분석 소요 시간 표시
  - 초기화 버튼으로 과거 결과(DB) 삭제
- Export 기본 경로 자동 설정(소스 폴더 선택 시 `<소스>/분류됨`)

## 윈도우 크기 제한
- `src-tauri/tauri.conf.json`에 최소 크기 설정:
  - `minWidth: 1000`, `minHeight: 600`

## 빌드/배포
- CLIP 모델 파일들을 Tauri 번들 리소스로 포함(onnx/*.onnx + tokenizer/config 파일)
- GitHub Actions로 mac/windows/linux 빌드 워크플로 추가: `.github/workflows/tauri-build.yml`
  - Linux arm64는 느려서 workflow_dispatch에서만 실행
- `.gitignore` 보강(빌드 산출물, rust target, 로컬 파일 등 포함)
