# CDP를 사용한 E2E 테스트 가이드

> Chrome DevTools Protocol(CDP)을 통해 rhwp-studio 편집기의 E2E 테스트를 자동 실행하고,
> 작업지시자가 Chrome 브라우저에서 테스트 과정을 실시간으로 시각적 확인할 수 있다.

---

## 1. 사전 준비

### 1.1 WASM 빌드

```bash
# Docker를 사용한 WASM 빌드
docker compose --env-file .env.docker run --rm wasm
```

빌드 결과물은 `pkg/` 폴더에 생성된다.

### 1.2 Chrome 디버깅 모드 시작 (Windows 호스트)

Windows CMD 또는 PowerShell에서 실행:

```cmd
"C:\Program Files\Google\Chrome\Application\chrome.exe" --remote-debugging-port=19222 --remote-debugging-address=0.0.0.0 --user-data-dir="C:\temp\chrome-debug"
```

| 옵션 | 설명 |
|------|------|
| `--remote-debugging-port=19222` | CDP 포트 (Puppeteer가 연결) |
| `--remote-debugging-address=0.0.0.0` | WSL2에서 접근 가능하도록 모든 인터페이스 바인딩 |
| `--user-data-dir` | 별도 프로필 (기존 Chrome과 충돌 방지) |

Chrome이 시작되면 빈 탭이 열린다. 테스트 실행 시 새 탭이 자동으로 열려 테스트 과정을 실시간으로 볼 수 있다.

### 1.3 WSL2 ↔ Windows 포트 포워딩 설정

Windows 11 호스트의 WSL2 Ubuntu에서 호스트 Chrome의 CDP에 접속하려면 **포트 프록시** 설정이 필요하다.

Windows **관리자 권한** PowerShell에서 실행:

```powershell
# WSL2 IP 확인 (Ubuntu 쪽에서: hostname -I)
# 예: 172.21.192.102

# 포트 프록시 추가
netsh interface portproxy add v4tov4 listenport=19222 listenaddress=0.0.0.0 connectport=19222 connectaddress=172.21.192.102

# 확인
netsh interface portproxy show v4tov4
```

| 항목 | 설명 |
|------|------|
| `listenport=19222` | Windows에서 수신할 포트 |
| `listenaddress=0.0.0.0` | 모든 인터페이스에서 수신 |
| `connectport=19222` | Chrome CDP 포트와 동일 |
| `connectaddress=172.21.192.102` | WSL2의 IP (재부팅 시 변경될 수 있음) |

Windows 방화벽이 활성화된 경우 인바운드 룰도 추가해야 한다:

```powershell
netsh advfirewall firewall add rule name="WSL2 CDP Proxy" dir=in action=allow protocol=TCP localport=19222 remoteip=172.21.192.102
```

> **주의**: WSL2 IP는 재부팅마다 변경된다. 변경 시 포트 프록시와 방화벽 룰을 재설정해야 한다.
> 아래 **1.3.1 WSL2 IP 고정** 설정을 적용하면 재설정이 불필요하다.
>
> ```powershell
> # 포트 프록시 재설정
> netsh interface portproxy delete v4tov4 listenport=19222 listenaddress=0.0.0.0
> netsh interface portproxy add v4tov4 listenport=19222 listenaddress=0.0.0.0 connectport=19222 connectaddress=<새 WSL2 IP>
>
> # 방화벽 룰 재설정
> netsh advfirewall firewall delete rule name="WSL2 CDP Proxy"
> netsh advfirewall firewall add rule name="WSL2 CDP Proxy" dir=in action=allow protocol=TCP localport=19222 remoteip=<새 WSL2 IP>
> ```

#### 1.3.1 WSL2 IP 고정 (권장)

WSL2의 IP를 고정하면 재부팅마다 포트 프록시와 방화벽 룰을 재설정할 필요가 없다.

**Windows 측 설정** — `C:\Users\<사용자>\.wslconfig` 파일 생성 또는 편집:

```ini
[wsl2]
networkingMode=static
ipAddress=172.21.192.102
gateway=172.21.192.1
```

> `networkingMode=static`은 Windows 11 22H2 이상 + WSL 2.0.0 이상에서 지원된다.
> `wsl --version`으로 WSL 버전을 확인할 수 있다.

**WSL2 Ubuntu 측 설정** — `/etc/wsl.conf` 파일에 DNS 설정 추가:

```ini
[network]
generateResolvConf = false
```

그리고 `/etc/resolv.conf`를 직접 생성:

```bash
sudo rm /etc/resolv.conf
echo "nameserver 8.8.8.8" | sudo tee /etc/resolv.conf
```

설정 후 PowerShell에서 WSL 재시작:

```powershell
wsl --shutdown
```

> **대안 (mirrored 모드)**: WSL 2.4.4+ 에서는 `networkingMode=mirrored`를 사용하면
> Windows와 WSL2가 동일한 IP를 공유하여 포트 프록시 자체가 불필요하다.
> 단, mirrored 모드는 일부 네트워크 환경에서 호환성 문제가 있을 수 있다.
>
> ```ini
> [wsl2]
> networkingMode=mirrored
> ```

### 1.4 Vite 개발 서버 시작 (WSL2)

```bash
cd rhwp-studio
npx vite --host 0.0.0.0 --port 7700 &
```

브라우저에서 `http://localhost:7700`으로 접속 가능한지 확인한다.

---

## 2. 테스트 실행

### 2.1 기본 실행

```bash
cd rhwp-studio
CHROME_CDP=http://172.21.192.1:19222 node e2e/edit-pipeline.test.mjs --mode=host
```

| 환경변수 | 설명 |
|---------|------|
| `CHROME_CDP` | Windows 호스트의 Chrome CDP 주소 |
| `--mode=host` | 호스트 Chrome에 CDP 연결 (기본값) |
| `--mode=headless` | WSL2 내부 headless Chrome 사용 (시각 확인 불가) |

> **IP 확인**: WSL2에서 Windows 호스트 IP는 `ip route show default | awk '{print $3}'`로 확인 가능.
> 일반적으로 `172.x.x.1` 형태이다.

### 2.2 사용 가능한 테스트

| 테스트 파일 | 설명 |
|------------|------|
| `e2e/edit-pipeline.test.mjs` | 편집 파이프라인 검증 (문단 추가/삭제, 표 삽입, 페이지 브레이크 등) |
| `e2e/text-flow.test.mjs` | 텍스트 플로우 (입력, 줄바꿈, 엔터, 페이지 넘김) |
| `e2e/page-break.test.mjs` | 페이지 브레이크 테스트 |
| `e2e/shape-inline.test.mjs` | 인라인 도형 테스트 |
| `e2e/typesetting.test.mjs` | 조판 테스트 |

### 2.3 headless 모드 (CI용)

시각적 확인 없이 자동 실행:

```bash
cd rhwp-studio
node e2e/edit-pipeline.test.mjs --mode=headless
```

headless 모드에서는 WSL2 내부의 Chromium을 사용하므로 Windows Chrome이 필요 없다.

---

## 3. 테스트 구조

### 3.1 테스트 케이스 규약

각 테스트 케이스는 새 문서를 생성하고, 첫 문단에 테스트 케이스 제목을 삽입한다:

```
TC #N: 테스트명
```

이를 통해 Chrome 브라우저에서 현재 어떤 테스트가 실행 중인지 시각적으로 확인할 수 있다.

### 3.2 edit-pipeline.test.mjs 테스트 케이스

| TC | 제목 | 검증 내용 |
|----|------|----------|
| #2 | 문단 추가 (Enter) | Enter로 3개 문단 생성, 텍스트 정합, 페이지 수 불변 |
| #3 | merge paragraph | Backspace로 문단 병합, 텍스트 결합 확인 |
| #4 | pagination | 50개 문단 생성 → 페이지 넘침 (2페이지+) |
| #5 | line wrap | 긴 텍스트 입력 → 자동 줄바꿈 |
| #6 | table insert | 텍스트 → 표(2x2) → 텍스트 구조, SVG 렌더링 |
| #7 | page break | 페이지 브레이크 삽입 → 페이지 수 증가 |
| #8 | vpos cascade | 문단 높이 변경 → 후속 문단 위치 전파 |
| #9 | stability | 분할/병합 5회 반복 후 텍스트/문단 수 보존 |

### 3.3 헬퍼 함수 (helpers.mjs)

| 함수 | 설명 |
|------|------|
| `launchBrowser()` | Chrome CDP 연결 또는 headless 시작 |
| `loadApp(page)` | Vite 서버에서 앱 로드 + WASM 초기화 대기 |
| `clickEditArea(page)` | 캔버스 클릭하여 편집 포커스 |
| `typeText(page, text)` | 키보드로 텍스트 입력 (글자별 30ms 지연) |
| `pressEnter(page)` | Enter 키 입력 |
| `screenshot(page, name)` | 스크린샷 저장 (`e2e/screenshots/`) |
| `getPageCount(page)` | WASM API로 페이지 수 조회 |
| `getParagraphCount(page)` | WASM API로 문단 수 조회 |
| `closeBrowser(browser)` | 브라우저 정리 |

### 3.4 WASM API 직접 호출

키보드 입력 외에 WASM API를 직접 호출하여 정밀한 편집 테스트를 수행할 수 있다:

```javascript
const result = await page.evaluate(() => {
  const w = window.__wasm;
  
  // 텍스트 삽입
  w.doc.insertText(0, 0, 0, 'Hello');
  
  // 문단 분할
  w.doc.splitParagraph(0, 0, 5);
  
  // 표 삽입
  const tr = JSON.parse(w.doc.createTable(0, 1, 0, 2, 2));
  
  // 셀 텍스트 삽입
  w.doc.insertTextInCell(0, tr.paraIdx, tr.controlIdx, 0, 0, 0, 'Cell');
  
  // 페이지 브레이크
  w.doc.insertPageBreak(0, 0, 5);
  
  // 문단 병합
  w.doc.mergeParagraph(0, 1);
  
  // 캔버스 재렌더링 트리거 (WASM API 직접 호출 후 필수)
  window.__eventBus?.emit('document-changed');
  
  return { pageCount: w.doc.pageCount() };
});
```

> **중요**: WASM API를 직접 호출한 후에는 반드시 `window.__eventBus?.emit('document-changed')`를
> 호출하여 캔버스를 갱신해야 화면에 반영된다. 키보드 입력(`typeText`, `pressEnter`)은 자동으로 처리된다.

---

## 4. 스크린샷

테스트 실행 시 각 단계의 스크린샷이 `rhwp-studio/e2e/screenshots/` 폴더에 저장된다:

```
e2e/screenshots/
  edit-01-split.png        # TC #2 결과
  edit-03-merge.png        # TC #3 결과
  edit-04-many-paragraphs.png
  edit-05-long-text.png
  edit-06-table-insert.png # TC #6 결과 (텍스트→표→텍스트)
  edit-07-page-break.png
  edit-08-vpos-cascade.png
  edit-09-stability.png
  edit-final.png           # 최종 상태
```

테스트 실패 시 해당 시점의 스크린샷으로 문제를 확인할 수 있다.

---

## 5. 새 테스트 추가 방법

### 5.1 기본 템플릿

```javascript
// ── N. 테스트 설명 ──
console.log('\n[N] 테스트 설명...');
await createNewDocument(page);
await clickEditArea(page);

const result = await page.evaluate(() => {
  const w = window.__wasm;
  if (!w?.doc) return { error: 'no doc' };
  try {
    // 제목 삽입
    w.doc.insertText(0, 0, 0, 'TC #N: 테스트명');
    w.doc.splitParagraph(0, 0, /* 제목 길이 */);
    
    // 테스트 로직...
    
    // 캔버스 갱신
    window.__eventBus?.emit('document-changed');
    
    return { /* 검증 데이터 */, ok: true };
  } catch (e) { return { error: e.message }; }
});
await page.evaluate(() => new Promise(r => setTimeout(r, 300)));

if (result.error) {
  console.log(`  SKIP: ${result.error}`);
} else {
  check(result.ok, `테스트 통과`);
  // 추가 검증...
}
await screenshot(page, 'edit-NN-name');
```

### 5.2 검증 패턴

| 패턴 | 사용 함수 |
|------|----------|
| 문단 텍스트 확인 | `w.doc.getTextRange(sec, para, offset, count)` |
| 셀 텍스트 확인 | `w.doc.getTextInCell(sec, para, ctrl, cell, cellPara, offset, count)` |
| 문단 수 확인 | `w.doc.getParagraphCount(sec)` |
| 페이지 수 확인 | `w.doc.pageCount()` |
| 줄 정보 확인 | `JSON.parse(w.doc.getLineInfo(sec, para, offset))` |
| SVG 렌더링 확인 | `w.doc.renderPageSvg(pageNum)` |

---

## 6. 트러블슈팅

### CDP 연결 실패

```
TypeError: Failed to fetch browser webSocket URL
```

- Chrome이 디버깅 모드로 실행 중인지 확인
- `CHROME_CDP` 환경변수의 IP/포트가 맞는지 확인
- Windows 방화벽이 해당 포트를 차단하지 않는지 확인

### 캔버스를 찾을 수 없음

```
Error: 캔버스를 찾을 수 없습니다
```

- Vite 개발 서버가 `0.0.0.0:7700`에서 실행 중인지 확인
- WASM 빌드(`pkg/`)가 최신인지 확인

### WASM API 호출 후 화면 미갱신

- `window.__eventBus?.emit('document-changed')` 호출 확인
- `await page.evaluate(() => new Promise(r => setTimeout(r, 300)))` 안정화 대기 추가

### 테스트가 타이밍 문제로 실패

- `typeText` 대신 `page.keyboard.type(text, { delay: 5 })`로 빠르게 입력
- WASM API 직접 호출로 전환 (키보드 입력보다 안정적)
- 안정화 대기 시간 증가 (`setTimeout` 값 조정)
