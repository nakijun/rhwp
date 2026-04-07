# 배포 가이드

rhwp 프로젝트의 배포 대상과 절차를 정리한다.

---

## 배포 대상

| 대상 | 패키지명 | 배포 위치 |
|------|---------|----------|
| VSCode 익스텐션 | rhwp-vscode | VS Code Marketplace + Open VSX |
| npm WASM 코어 | @rhwp/core | npmjs.com |
| npm 에디터 | @rhwp/editor | npmjs.com |

---

## 1. 사전 준비

### 1.1 토큰 설정

프로젝트 루트의 `.env` 파일에 다음 토큰이 필요하다:

```
VSCE_PAT=<Azure DevOps Personal Access Token>
OVSX_PAT=<Open VSX Access Token>
npm_token=<npm Access Token>
```

| 토큰 | 발급처 | 권한 |
|------|--------|------|
| VSCE_PAT | [Azure DevOps](https://dev.azure.com) → Personal Access Tokens | Marketplace (Publish) |
| OVSX_PAT | [open-vsx.org](https://open-vsx.org) → Access Tokens | Publish |
| npm_token | [npmjs.com](https://www.npmjs.com) → Access Tokens | Publish |

npm 토큰은 `~/.npmrc`에도 등록해야 한다:

```bash
echo "//registry.npmjs.org/:_authToken=<npm_token>" > ~/.npmrc
```

### 1.2 WASM 빌드

```bash
docker compose --env-file .env.docker run --rm wasm
```

빌드 결과: `pkg/rhwp_bg.wasm`, `pkg/rhwp.js`, `pkg/rhwp.d.ts`

### 1.3 로고 파일 확인

```bash
ls assets/logo/logo-128.png  # VSCode 익스텐션 아이콘
```

---

## 2. VSCode 익스텐션 배포

### 2.1 버전 업데이트

`rhwp-vscode/package.json`:
```json
"version": "0.6.0"
```

`rhwp-vscode/CHANGELOG.md`에 새 버전 항목 추가.

### 2.2 README 점검

`rhwp-vscode/README.md` 확인 사항:
- 기능 목록이 최신인지
- Third-Party Licenses 섹션 존재
- Trademark 면책 조항 존재
- Notice (한컴 공개 문서 참고) 문구 존재

### 2.3 배포 실행

```bash
cd rhwp-vscode
bash publish.sh
```

`publish.sh`가 자동으로 수행하는 작업:
1. `assets/logo/logo-128.png` → `media/icon.png` 복사
2. `pkg/rhwp_bg.wasm`, `pkg/rhwp.js` → `media/` 복사
3. webpack 빌드
4. VS Code Marketplace 배포 (`npx vsce publish`)
5. Open VSX 배포 (`npx ovsx publish`)

### 2.4 배포 확인

- https://marketplace.visualstudio.com/items?itemName=edwardkim.rhwp-vscode
- https://open-vsx.org/extension/edwardkim/rhwp-vscode

---

## 3. npm @rhwp/core 배포

### 3.1 패키지 준비

```bash
bash scripts/prepare-npm.sh
```

이 스크립트가 수행하는 작업:
1. `Cargo.toml`에서 버전 읽기
2. `pkg/package.json` 생성 (`@rhwp/core`, 버전, 메타데이터)
3. `npm/README.md` → `pkg/README.md` 복사

### 3.2 README 점검

`npm/README.md` 확인 사항:
- 빠른 시작 가이드 (코드 예시)
- measureTextWidth 설명
- **폰트 설정 가이드** (SVG와 폰트 관계, 폴백 매핑 테이블, CDN/셀프호스팅)
- Third-Party Licenses 섹션
- Trademark 면책 조항

### 3.3 배포 실행

```bash
cd pkg
npm publish --access public
```

### 3.4 배포 확인

- https://www.npmjs.com/package/@rhwp/core

---

## 4. npm @rhwp/editor 배포

### 4.1 버전 업데이트

`npm/editor/package.json`:
```json
"version": "0.6.3"
```

### 4.2 README 점검

`npm/editor/README.md` 확인 사항:
- 3줄 빠른 시작 예시
- API 문서 (createEditor, loadFile, pageCount, getPageSvg, destroy)
- **폰트 안내** (내장 폴백 폰트 목록, 품질 설명, 셀프 호스팅 시 폰트)
- Third-Party Licenses (Rust 크레이트 + 내장 웹 폰트 + 프론트엔드)
- Trademark 면책 조항

### 4.3 배포 실행

```bash
cd npm/editor
npm publish --access public
```

### 4.4 배포 확인

- https://www.npmjs.com/package/@rhwp/editor

---

## 5. 배포 후 작업

### 5.1 Git 커밋

버전 업데이트, CHANGELOG, README 변경사항을 커밋한다.

```bash
git add rhwp-vscode/package.json rhwp-vscode/CHANGELOG.md rhwp-vscode/README.md
git add npm/README.md npm/editor/README.md npm/editor/package.json
git commit -m "v0.6.0 릴리즈: VSCode 익스텐션 + npm 패키지 배포"
```

### 5.2 devel/main push

```bash
git checkout devel && git merge local/devel && git push origin devel
git checkout main && git merge devel && git push origin main
```

### 5.3 GitHub Release 생성 (선택)

```bash
git tag v0.6.0
git push origin v0.6.0
gh release create v0.6.0 --title "v0.6.0 — 제목" --notes "릴리즈 노트"
```

---

## 6. 배포 체크리스트

배포 전 확인 항목:

- [ ] `cargo build` + `cargo test` 통과
- [ ] WASM 빌드 완료 (`pkg/`)
- [ ] E2E 테스트 통과
- [ ] 저작권 폰트가 포함되지 않았는지 확인
- [ ] package.json 버전 업데이트
- [ ] CHANGELOG.md 작성
- [ ] README.md 현행화 (기능, 폰트 가이드, 라이선스, 상표)
- [ ] `.env` 토큰 확인 (VSCE_PAT, OVSX_PAT, npm_token)
- [ ] 배포 후 각 마켓플레이스에서 페이지 확인

---

## 7. 트러블슈팅

### VSCE_PAT 오류

```
❌ VSCE_PAT가 .env에 설정되지 않았습니다
```

- `.env` 파일에서 `VSCE_PAT=` 줄 앞에 개행이 있는지 확인
- Windows 줄바꿈(`\r`)이 포함되었을 수 있음: `cat -A .env`로 확인

### npm publish 버전 충돌

```
You cannot publish over the previously published versions
```

- 이미 배포된 버전. package.json 버전을 올려야 함 (예: 0.6.0 → 0.6.1)
- npm은 한 번 배포된 버전을 덮어쓸 수 없음

### pkg/ 권한 오류

```
Permission denied: pkg/package.json
```

- Docker 빌드로 `pkg/`가 root 소유로 생성된 경우
- `sudo chown -R $(whoami) pkg/` 로 소유권 변경 후 재시도

### Open VSX 배포 실패

- OVSX_PAT 토큰 만료 확인 (open-vsx.org에서 재발급)
- `npx ovsx publish` 수동 실행으로 에러 메시지 확인
