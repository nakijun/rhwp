# Task 404: VSCode 확장 Marketplace 게시 준비

## 수행 목표

rhwp-vscode 패키지를 VSCode Marketplace에 게시할 수 있도록 필수 자산 및 메타데이터를 보완한다.

## 작업 항목

| 항목 | 설명 |
|------|------|
| 아이콘 | 128x128 PNG 확장 아이콘 생성 (`media/icon.png`) |
| README.md | Marketplace 설명 페이지 (기능 소개, 스크린샷, 사용법) |
| CHANGELOG.md | 버전별 변경 이력 (v0.1.0 초기 릴리즈) |
| LICENSE | MIT 라이선스 파일 복사 |
| package.json | `repository`, `icon`, `keywords`, `bugs` 필드 보완 |
| .vscodeignore | `.vscode/`, 불필요 파일 제외 추가 |
| vsce package 검증 | `.vsix` 패키징 테스트 |

## 구현 단계

### 1단계: 메타데이터 및 문서 작성

- `rhwp-vscode/README.md` 작성 (기능, 사용법, 빌드 방법)
- `rhwp-vscode/CHANGELOG.md` 작성
- `rhwp-vscode/LICENSE` 루트에서 복사
- `package.json` 필드 보완

### 2단계: 아이콘 생성

- SVG로 아이콘 디자인 → 128x128 PNG 변환
- `rhwp-vscode/media/icon.png` 배치

### 3단계: 패키징 검증

- `.vscodeignore` 정리
- `vsce package` 실행 → `.vsix` 생성 확인
- 경고/오류 해결

## 승인 요청

위 수행계획서를 검토 후 승인 부탁드립니다.
