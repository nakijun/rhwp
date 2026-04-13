**HWP 파일 시맨틱 청킹 가이드**  
**데이터 엔지니어를 위한 실전 매뉴얼 (rhwp 기반, 2026년 4월 13일 업데이트)**

**버전**: v2026.04.13  
**목적**: 한국 공공기관·기업 환경에서 대량 HWP/HWPX 문서를 **GIGO 없이** RAG에 활용할 수 있도록, **시맨틱 구조(표 OLAP, 컨트롤 계층, 책갈피, 날짜 필드 등)를 최대한 보존**한 청킹 파이프라인 구축 방법을 제시합니다.  
**대상**: 데이터 엔지니어, RAG 개발자, 한국형 문서 AI 프로젝트 담당자

### 1. 서론: GIGO 원칙과 HWP RAG의 현실
**“Garbage In, Garbage Out”** — AI 분야에서 가장 중요한 명언입니다.  
HWP RAG에서 입력 데이터의 대부분 문제는 **파싱 단계**에서 발생합니다.

- 일반 로더(라마인덱스 기본 HWP 로더, PDF 변환 → Markdown): 복잡한 표 병합·중첩, 다단 레이아웃, 책갈피, 자동 날짜 코드, OLAP 수준 시맨틱이 파괴되어 hallucination이 심해집니다.
- rhwp 개발자(edwardkim)가 Discussion #113에서 지적한 핵심: 공공 HWP의 표는 단순 데이터가 아니라 **OLAP 수준의 함축적 의미**를 담고 있으며, Markdown이나 PDF는 이를 제대로 표현할 수 없습니다. 따라서 **HWP 포맷 자체를 네이티브 시맨틱 파싱**해야 합니다.

**rhwp**는 Rust + WebAssembly 기반 오픈소스 파서로, 이 문제를 정공법으로 해결합니다.  
`rhwp dump` 명령은 조판 구조를 상세히 드러내 데이터 엔지니어가 **시맨틱 청킹 전략을 설계**할 수 있게 해줍니다.

**최신 정보 (2026.4.13 기준)**:  
- rhwp v0.7.0 (2026년 4월 11일 릴리스) — 조판 레이아웃 품질 대폭 개선, 브라우저 확장 지원 추가.  
- 지원: HWP 5.0 (OLE2 바이너리) + HWPX  
- 라이선스: MIT (상용/공공 자유 사용, 한컴과 무관)

### 2. rhwp 설치 및 기본 준비
```bash
# Rust 설치 후
cargo install rhwp-cli   # 또는 소스 빌드

# WASM 웹 에디터 (디버깅용)
cd rhwp && npm install && npm run dev
```

- 출력: SVG, Canvas 렌더링, dump 텍스트, Web Editor
- 에어갭 환경에도 적합 (WASM + 로컬 실행)

### 3. dump 명령: 시맨틱 청킹의 출발점
`rhwp dump`는 HWP 내부 **조판부호(컨트롤) 계층 구조**를 텍스트로 완전히 드러내는 디버깅 도구입니다. 단순 텍스트 추출이 아니라 위치·속성·계층 정보를 제공합니다.

**기본 사용법**
```bash
rhwp dump 파일.hwp                    # 전체
rhwp dump 파일.hwp -s 0               # 구역 0만
rhwp dump 파일.hwp -s 0 -p 3          # 구역 0의 문단 3만 (문제 부분 집중)
rhwp dump-pages 파일.hwp -p 15        # 페이지 16 레이아웃 아이템
```

**주요 출력 요소**
- **구역 헤더**: 용지 크기, 여백, 방향 (mm + HWPUNIT 병기)
- **문단 헤더**: 문자 수, 컨트롤 개수, 단/쪽 나누기 정보
- **컨트롤 상세**:
  - 표: 행×열, 셀 수, 쪽나눔 여부
  - 도형/그림: 위치 기준(Paper/Page/Column/Para), 배치 방식(Square, TopAndBottom, BehindText 등), z-order, scale/offset, 회전
  - 책갈피: 이름과 위치
  - 날짜 필드/코드: 형식 문자열, 코드 배열 (년/월/일 등)
  - 머리말/꼬리말, 하이퍼링크, 각주 등

**단위 환산** (dump에 자주 등장):
- HWPUNIT → mm: `hu × 25.4 / 7200`
- HWPUNIT → px (96DPI): `hu × 96 / 7200`

이 dump를 파싱하면 **rich metadata**를 붙인 구조화된 JSON을 만들기 쉽습니다.

### 4. HWP 내부 시맨틱 구조 예시
(사용자 제공 디버깅 레이아웃 이미지 기반)

- **책갈피 (10.2)**: 파일 앞쪽 정보 블록에 List 저장 → 상호 참조로 사용. dump에서 `[0] 책갈피: "name"` 형태로 출력.
- **날짜 형식 (10.3)**: `hchar array[40]`로 형식 문자열 저장.
- **날짜 코드 (10.4)**: `word array[2]` 등으로 날짜 정보 + 형식 코드 (0=자릿수 메움, 1=년, 2=월 … 6=요일 한자). 자동 갱신 필드로 메타데이터화 가능.

이 정보를 활용하면 “이 표는 특정 책갈피와 연결된 날짜 필드를 포함한다” 같은 **시맨틱 관계**를 청크에 보존할 수 있습니다.

### 5. 시맨틱 청킹 파이프라인 (GIGO 방지 설계)
```mermaid
graph TD
    A[HWP 파일] --> B[rhwp dump 또는 parse]
    B --> C[Dump/구조 파싱 (Python/Rust)]
    C --> D[계층적 JSON + Rich Metadata]
    D --> E[시맨틱 청킹 (표 OLAP, 컨트롤 단위, 위치 기반)]
    E --> F[하이브리드 Vector DB + 구조 검색]
    F --> G[RAG Retrieval & Generation]
```

**추천 청킹 전략**
1. 문단 단위 기본 + 컨트롤 메타데이터 병합
2. **표 OLAP 청킹**: 병합 셀·중첩 표를 하나의 JSON 객체로 (행/열 계층 + 좌표 정보)
3. 책갈피/날짜 필드 → 엔티티/링크 청크 (temporal/reference metadata 추가)
4. 위치 기반 청킹: HorzRelTo / VertRelTo 활용
5. 하이브리드 검색: 벡터 유사도 + SQL-like 구조 쿼리

**중간 표현 예시 (JSON)**
```json
{
  "section": 0,
  "para": 3,
  "text": "...",
  "controls": [
    {"type": "table", "rows": 3, "cols": 11, "semantic": "OLAP", "cells": 28},
    {"type": "bookmark", "name": "project_end", "linked_date_code": {...}}
  ],
  "metadata": {
    "position": {"horz": "Paper", "vert": "Page"},
    "temporal": "auto_date_field"
  }
}
```

### 6. HWP 파서 비교 (2026년 4월 기준, RAG 적합성 중심)
| 파서 이름                  | 언어 / 방식             | 지원 포맷          | 시맨틱 구조 보존 (표·컨트롤) | 디버깅 기능          | 크로스 플랫폼 | RAG 적합성          | 주요 강점 / 약점                                      | 추천 시나리오                     |
|---------------------------|-------------------------|-------------------|-----------------------------|---------------------|--------------|---------------------|-----------------------------------------------------|----------------------------------|
| **rhwp** (edwardkim)     | Rust + WASM            | HWP 5.0 + HWPX   | ★★★★★ (OLAP 표, 좌표·계층) | 최고 (`rhwp dump`) | 최고 (WASM, 에어갭) | 최고               | 강점: 네이티브 시맨틱 + 디버깅 최적<br>약점: 자동 파이프라인은 직접 구축 | 복잡 공공 문서, 시맨틱 RAG      |
| **hwp-rs** / **hwpers**  | Rust                   | HWP 중심         | ★★★★☆                      | 좋음               | 좋음        | 높음               | 안정적 로우레벨 파싱                               | Rust 백엔드                      |
| **python-hwpx**          | Pure Python            | HWPX             | ★★★☆                       | 없음               | 최고        | 중                 | 한/글 설치 불필요, 빠른 프로토타입                | HWPX 중심 간단 프로젝트         |
| **kordoc** / markitdown-hwp | TS/Node, Python+Docpler | HWP + 다중 포맷  | ★★★☆ (Markdown 중심)      | 없음               | 좋음        | 중                 | 빠른 Markdown 출력, 표 기본 지원                  | 빠른 변환 후 RAG                 |
| **한컴 Data Loader**     | SDK (상용)             | HWP + HWPX + PDF/OFFICE | ★★★★☆~★★★★★               | 제한적             | 중          | 높음 (유료)        | 공식 정확도 최고, RAG 전용 설계                    | 예산 충분한 기업/공공 프로젝트  |
| **기타 (Upstage 등)**    | 클라우드 API           | 다중 포맷         | 높음 (VLM 보조)            | 없음               | 최고        | 높음               | 속도 빠름, 복잡 레이아웃 강점                     | 클라우드 중심, VLM 보조         |

**파서 선택 팁**:
- 시맨틱 청킹 최우선 → **rhwp** (dump 기반 커스텀 파이프라인)
- 최고 정확도 + 예산 있음 → **한컴 Data Loader**
- 빠른 프로토타입 → kordoc 또는 python-hwpx
- 반드시 **파싱 정확도 테스트** (복잡 표 샘플 10~20개로 dump 비교 + retrieval 평가) 수행

### 7. 실전 구현 팁 (데이터 엔지니어 관점)
- dump 후처리: Python 스크립트로 텍스트 파싱 → Pydantic 모델 구조화
- 테스트 루프: 복잡 문서 샘플 → dump → 청킹 → RAGAS faithfulness 평가 반복
- 에어갭: rhwp WASM + 로컬 Vector DB (Qdrant/Chroma)
- VLM 보조 (선택): rhwp SVG 내보내기 → Upstage Document Parse 또는 PaddleOCR-VL 결합
- 평가 지표: Retrieval Accuracy, Hallucination Rate, Faithfulness

### 8. 베스트 프랙티스 & 주의사항
- 절대 피할 것: “간단 라마인덱스 로더 + 에이전트 하나로 끝” 접근 → GIGO 직행
- 필수 과정: 파싱 정확도 테스트 → dump 기반 구조 분석 → 청킹 전략 튜닝 반복
- rhwp는 강력한 **디버깅/기반 도구**입니다. 완전 자동 RAG 파이프라인은 dump를 활용해 여러분이 직접 구축하세요.
- 최신 업데이트: rhwp GitHub에서 조판 품질 지속 개선 중 (v0.7.0 기준)
