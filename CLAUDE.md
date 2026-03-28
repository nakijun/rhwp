# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 프로젝트 개요

**목표**: Rust 언어로 HWP 파일 뷰어/에디터 개발
- Rust로 HWP 파일 파서 및 렌더러 구현
- WebAssembly(WASM)로 빌드하여 웹브라우저에서 HWP 문서를 볼 수 있도록 함
- 한컴 웹기안기의 오픈소스 대안

## 문서 생성 규칙

모든 문서는 한국어로 작성한다.

문서 폴더 구조 (`mydocs/` 하위):
- `orders/` - 오늘 할일 문서 (yyyymmdd.md)
- `plans/` - 수행 계획서, 구현 계획서 (task_{number}.md)
- `plans/archives/` - 완료된 계획서 보관
- `working/` - 단계별 완료 보고서
- `report/` - 기본 보고서
- `feedback/` - 피드백 저장
- `tech/` - 기술 사항 정리 문서
- `manual/` - 매뉴얼, 가이드 문서
- `troubleshootings/` - 트러블슈팅 관련 문서

## 빌드 및 실행

### 로컬 빌드

```bash
cargo build                    # 네이티브 빌드
cargo test                     # 테스트 실행
cargo build --release          # 릴리즈 빌드
```

네이티브 빌드·테스트·SVG 내보내기는 **항상 로컬 cargo**를 사용한다.

### Docker 빌드 (WASM 전용)

```bash
cp .env.docker.example .env.docker   # 최초 1회: 환경변수 설정
docker compose --env-file .env.docker run --rm wasm    # WASM 빌드 (→ pkg/)
```

Docker는 **WASM 빌드 전용**으로만 사용한다. 네이티브 빌드/테스트에는 사용하지 않는다.

### SVG 내보내기

```bash
rhwp export-svg sample.hwp                         # output/ 폴더에 SVG 출력
rhwp export-svg sample.hwp -o my_dir/              # 지정 폴더에 출력
rhwp export-svg sample.hwp -p 0                    # 특정 페이지만 출력 (0부터)
rhwp export-svg sample.hwp --show-para-marks       # 문단부호(↵/↓) 표시
rhwp export-svg sample.hwp --show-control-codes    # 조판부호 표시 (문단부호+개체마커)
rhwp export-svg sample.hwp --debug-overlay         # 디버그 오버레이 (문단/표 경계+인덱스)
```

#### 디버그 오버레이 (`--debug-overlay`)

문단/표의 경계와 인덱스를 SVG에 시각적으로 표시한다.

- **문단**: 색상 교대 점선 경계 + `s{섹션}:pi={인덱스} y={좌표}` 라벨 (좌측 상단)
- **표**: 빨간 점선 경계 + `s{섹션}:pi={인덱스} ci={컨트롤} {행}x{열} y={좌표}` 라벨 (우측 상단)
- 셀 내부 문단, 머리말/꼬리말/바탕쪽/각주 영역은 제외

#### 페이지네이션 결과 덤프 (`dump-pages`)

특정 페이지의 문단/표 배치 목록과 높이를 확인한다.

```bash
rhwp dump-pages sample.hwp -p 15    # 페이지 16 (0부터) 배치 결과
```

출력 예시:
```
=== 페이지 16 (global_idx=15, section=2, page_num=6) ===
  body_area: x=96.0 y=103.6 w=601.7 h=930.5
  단 0 (items=7)
    FullParagraph  pi=41  h=37.3 (sb=16.0 lines=21.3 sa=0.0)  "자료형 설명"
    Table          pi=45 ci=0  16x4  492.2x278.7px  wrap=TopAndBottom tac=false
```

### IR 덤프 (`dump`)

문서의 조판부호 구조를 덤프한다. 섹션/문단 필터를 지정하여 특정 문단의 ParaShape, LINE_SEG, 표 속성을 확인할 수 있다.

```bash
rhwp dump sample.hwp                  # 전체 구조 덤프
rhwp dump sample.hwp -s 2 -p 45      # 섹션 2, 문단 45만 덤프
```

출력 예시:
```
--- 문단 2.45 --- cc=9, text_len=0, controls=1
  [PS] ps_id=32 align=Justify spacing: before=1000 after=0 line=160/Percent
       margins: left=7000 right=4000 indent=0 border_fill_id=1
  ls[0]: vpos=15360, lh=1000, th=1000, bl=850, ls=600, cs=3500, sw=0
  [0] 표: 16행×4열
  [0]   [common] treat_as_char=false, wrap=위아래, vert=문단(0=0.0mm)
  [0]   [outer_margin] left=1.0mm top=2.0mm right=1.0mm bottom=7.0mm
```

### 디버깅 워크플로우

레이아웃/간격 버그 디버깅 시 다음 순서로 진행한다:

1. `export-svg --debug-overlay` → SVG에서 문단/표 식별 (`s{섹션}:pi={인덱스} y={좌표}`)
2. `dump-pages -p N` → 해당 페이지의 문단 배치 목록과 높이 확인
3. `dump -s N -p M` → 특정 문단의 ParaShape, LINE_SEG, 표 속성 상세 조사

코드 수정 없이 전 과정 수행 가능하다.

### HWPUNIT

- 1인치 = 7200 HWPUNIT
- 1인치 = 25.4 mm

### 예제 폴더

- `samples/` - 테스트용 HWP 파일

### 출력 폴더

- `output/` - 렌더링 결과물 (SVG, HTML 등) 기본 출력 폴더
- `.gitignore`에 등록되어 있으므로 Git에 포함되지 않음

### E2E 테스트

E2E 테스트는 Puppeteer (puppeteer-core) 기반이며, 두 가지 모드로 실행할 수 있다.

#### headless Chrome (자동화용)

```bash
cd rhwp-studio
npx vite --host 0.0.0.0 --port 7700 &   # Vite dev server
node e2e/text-flow.test.mjs              # 텍스트 플로우 테스트
```

#### 호스트 Chrome CDP (시각 확인용)

1. Chrome 실행 (원격 디버깅 활성화):
```
chrome --remote-debugging-port=9222 --remote-debugging-address=0.0.0.0 --remote-allow-origins=*
```

2. 테스트 실행:
```bash
cd rhwp-studio
npx vite --host 0.0.0.0 --port 7700 &
node e2e/text-flow.test.mjs --mode=host
```

## rhwp-studio UI 명칭 규약

코드와 대화에서 혼동을 방지하기 위해, 아래 명칭을 통일하여 사용한다.

```
┌─────────────────────────────────────────────────┐
│  메뉴바 (#menu-bar)                              │
│  파일 | 편집 | 보기 | 입력 | 서식 | 쪽 | 표      │
├─────────────────────────────────────────────────┤
│  도구 상자 (#icon-toolbar)                        │
│  [오려두기][복사][붙이기] | [글자모양][문단모양] | … │
├─────────────────────────────────────────────────┤
│  서식 도구 모음 (#style-bar)                      │
│  [스타일▼][글꼴▼][크기] | 가가간가 | ◀ ≡ ▶ ≡≡ | ⇕  │
├─────────────────────────────────────────────────┤
│                                                 │
│  편집 영역 (#scroll-container)                    │
│                                                 │
├─────────────────────────────────────────────────┤
│  상태 표시줄 (#status-bar)                        │
│  1/1쪽 | 구역:1/1 | 삽입 |           100% [−][+] │
└─────────────────────────────────────────────────┘
```

| 한국어 명칭 | HTML id/class | 설명 |
|------------|---------------|------|
| 메뉴바 | `#menu-bar` | 최상단 드롭다운 메뉴 (파일/편집/보기/입력/서식/쪽/표) |
| 도구 상자 | `#icon-toolbar` | 아이콘+라벨 버튼 모음 (tb-btn, tb-group) |
| 서식 도구 모음 | `#style-bar` | 스타일/글꼴/크기/서식 버튼 (sb-btn, sb-combo) |
| 편집 영역 | `#scroll-container` | 문서 페이지 렌더링 + 스크롤 영역 |
| 상태 표시줄 | `#status-bar` | 하단 쪽/구역/모드/줌 표시 |

### CSS 접두어 규칙

| 접두어 | 대상 |
|--------|------|
| `tb-` | 도구 상자 (#icon-toolbar) 요소 |
| `sb-` | 서식 도구 모음 (#style-bar) 요소 |
| `stb-` | 상태 표시줄 (#status-bar) 요소 |
| `md-` | 메뉴바 드롭다운 (#menu-bar) 요소 |
| `dialog-` | 대화상자 공통 |
| `cs-` | 글자모양 대화상자 (char-shape) |
| `ps-` | 문단모양 대화상자 (para-shape) |

## 워크플로우

### 브랜치 관리

| 브랜치 | 용도 |
|--------|------|
| `main` | 최종 릴리즈 |
| `devel` | 개발 통합 |
| `local/task{num}` | 타스크별 작업 |

### 타스크 진행 절차

1. `mydocs/orders/`에 등록된 타스크 중 작업지시자가 지정한 타스크 수행. 타스크 브랜치 스위치 후 다음 단계 진행.
2. 수행 전 수행계획서 작성 → 승인 요청
3. 구현 계획서 작성 (최소 3단계, 최대 6단계) → 승인 요청
4. 단계별 진행 시작
5. 각 단계 완료 후 단계별 완료보고서 작성 → 승인 요청
6. 승인 후 다음 단계 진행
7. 모든 단계 완료 시 최종 결과 보고서 작성 → 승인 요청
8. 승인 요청 시 작업지시자가 피드백 문서를 `mydocs/feedback/`에 등록
9. 모든 테스트 통과 시 피드백 없음
10. 최종 결과보고서 작성 후 오늘할일 해당 타스크 상태 갱신
