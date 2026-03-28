# HWP Viewer for Visual Studio Code

VSCode에서 HWP/HWPX 문서를 바로 열어볼 수 있는 확장 프로그램입니다.

[rhwp](https://github.com/edwardkim/rhwp) 프로젝트의 WebAssembly 렌더링 엔진을 기반으로, 한컴오피스 한글 문서를 별도 프로그램 없이 VSCode 안에서 확인할 수 있습니다.

## 기능

- HWP/HWPX 파일 더블클릭으로 바로 열기
- Canvas 2D 기반 고품질 문서 렌더링
- 가상 스크롤 (대용량 문서 지원)
- 줌 인/아웃 (Ctrl+마우스 휠 또는 상태 표시줄 버튼)
- 페이지 네비게이션 (상태 표시줄에 현재 쪽 표시)
- 문서 내 이미지 렌더링

## 지원 형식

| 확장자 | 설명 |
|--------|------|
| `.hwp` | 한컴오피스 한글 문서 (바이너리) |
| `.hwpx` | 한컴오피스 한글 문서 (OOXML 기반) |

## 사용법

1. 확장을 설치합니다.
2. VSCode에서 `.hwp` 또는 `.hwpx` 파일을 엽니다.
3. 문서가 자동으로 HWP Viewer에서 렌더링됩니다.
4. 스크롤하여 페이지를 탐색합니다.
5. Ctrl+마우스 휠 또는 하단 상태 표시줄의 +/- 버튼으로 줌을 조절합니다.

별도 프로그램 설치나 설정 없이 바로 사용할 수 있습니다.

## 개발자용

소스에서 직접 빌드하려면 WASM 빌드가 선행되어야 합니다 (`pkg/` 디렉토리에 `rhwp_bg.wasm`, `rhwp.js` 필요).

```bash
cd rhwp-vscode
npm install
npm run compile
```

## 라이선스

MIT License - [LICENSE](LICENSE)
