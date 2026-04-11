# Task #102 구현계획서 전문가 검토 피드백

> **작성일**: 2026-04-11
> **검토 대상**: `mydocs/plans/task_m100_102_impl.md` (레이어 1, 레이어 2 방어 로직)
> **검토자**: 워드프로세서 페이지 레이아웃 엔진 전문 에이전트

---

## 요약 테이블

| 항목 | 레이어 1 | 레이어 2 | 두 레이어 동시 |
|------|----------|----------|----------------|
| 무한 루프 위험 | 낮음 (가드 존재), 취약점 잠재 | 없음 | 이중 이동 가능성 존재 |
| `current_height` 보정 | 불필요 | **누락 시 심각한 오류** | 중복 보정 위험 |
| 단 전환 처리 | 해당 없음 | carry 재삽입 누락 | 영향 없음 |
| `page_vpos_base` 처리 | 해당 없음 | carry 재삽입 시 누락 위험 | 해당 없음 |
| `line_count == 1` | 실질 효과 없음 | 정상 작동 | 해당 없음 |
| 빈 페이지 overflow | 안전 (가드) | 안전 (가드) | 안전 |
| 쪽 나누기 경로 | 해당 없음 | 미작동 (의도적, 올바름) | 해당 없음 |
| 성능 | 문제없음 | 문제없음 | 문제없음 |

---

## 1. 무한 루프 위험

### 레이어 1 단독 — 낮음, 잠재 취약점 존재

`cursor_line == 0` 및 `!current_items.is_empty()` 가드가 안전망 역할을 한다.
`advance_column_or_new_page()` 후 새 페이지에서 `current_height = 0`으로 리셋되어
`guard_zone_start` 조건이 통상 거짓이 되어 루프를 탈출한다.

**취약 경로**: 각주가 페이지 거의 전체를 점유하여 `available_now`가 극히 작은 경우,
`guard_zone_start`가 음수가 되어 새 페이지에서도 조건이 참이 될 수 있다.
현재 `!current_items.is_empty()` 가드로 막히지만, 미래 코드 변경 시 취약점이 될 수 있다.

**권장**: 루프 내 `advance_column_or_new_page()` + `continue` 경로에 반복 횟수 상한(`guard_advance_count`) 추가.

### 레이어 2 단독 — 없음

`advance_column_or_new_page` → `flush_column` → `push_new_page` 경로에서 재귀 호출 없음.
`overflow_carry.is_some()` 가드와 `current_items.last()` 확인이 이중 안전망으로 작동.

### 두 레이어 동시 작동 — 이중 이동 가능성

**실제 충돌 경로 존재**: 레이어 1이 `cursor_line == 0`에서 `advance_column_or_new_page()`를 호출하면,
레이어 2의 `check_last_item_overflow`가 *직전 반복에서 배치된 다른 항목*을 carry로 꺼낼 수 있다.
이후 레이어 1의 `continue`로 새 페이지에서 같은 문단 줄을 재배치하면 **항목 순서 역전**이 발생할 수 있다.

**권장**: 레이어 간 활성화 상호 배제 — 레이어 1이 advance를 호출하는 경로에서 레이어 2가 작동하지 않도록
플래그 또는 적용 범위 분리.

---

## 2. 의도하지 않은 레이아웃 오류

### ⚠️ 심각: 레이어 2 carry 재삽입 시 `current_height` 미보정

현재 설계: "current_height 보정은 flush 후 current_height=0 리셋에 의존"

이것은 **심각한 결함**이다. 새 페이지에서 `current_height = 0` 리셋 후 carry 항목을 재삽입했는데
`current_height`에 carry 항목의 높이를 더하지 않으면, 이후 모든 `available_height()` 계산이
carry 항목이 차지한 공간을 무시한다.

**필수 수정**: carry 재삽입 시 반드시 `current_height += carry_item_height` 함께 수행.

```rust
if let Some(carry_item) = self.overflow_carry.take() {
    let carry_h = self.compute_item_height(&carry_item, measured);
    self.current_items.push(carry_item);
    self.current_height += carry_h;
    // page_vpos_base도 carry 항목 기준으로 설정
}
```

### ⚠️ 중요: 단 전환 경로에서 carry 재삽입 누락

현재 설계는 `push_new_page` 직후 carry 재삽입을 수행하는 구조인데,
단 전환 경로(`current_column + 1 < col_count`)에서는 `push_new_page`가 호출되지 않아
carry 재삽입 로직이 작동하지 않는다. **overflow된 항목이 carry에 영구적으로 갇혀 배치되지 않는다.**

**필수 수정**: 단 전환 분기에도 carry 재삽입 처리 추가:

```rust
pub fn advance_column_or_new_page(&mut self) {
    self.check_last_item_overflow();
    self.flush_column();
    if self.current_column + 1 < self.col_count {
        self.current_column += 1;
        self.current_height = 0.0;
        self.reinsert_carry_if_any();  // 단 전환 후에도 carry 재삽입
    } else {
        self.push_new_page();  // push_new_page 내에서 reinsert_carry_if_any 호출
    }
}
```

### `page_vpos_base` 누락 위험

carry 재삽입 후 `page_vpos_base`가 None으로 남으면 vpos 보정이 무효화된다.
carry된 항목(PartialParagraph/FullParagraph)의 `line_segs` 인덱스 기준으로
`page_vpos_base` 설정 로직을 carry 재삽입 코드에 포함해야 한다.

### `PartialParagraph` `start_line`/`end_line` 유효성

carry 후에도 `start_line`/`end_line`은 원본 문단의 줄 인덱스 그대로 유효하다.
단, 레이어 2가 while 루프 진행 중 carry를 삽입하는 경우 루프의 `cursor_line`과
carry된 PartialParagraph의 `start_line` 간 정합성 문제가 생길 수 있으므로 주의.

### 레이어 1의 `cursor_line == 0` 조건 한계

`cursor_line == 0`은 "이 문단의 첫 줄 배치"를 의미하며, "이 페이지/단에 처음 배치"와 다를 수 있다.
`cursor_line > 0`인 연속 페이지의 PartialParagraph 배치에는 guard가 작동하지 않는다.
바로 그 계산 오차가 이 방어 로직의 존재 이유이므로 논리적 완결성 부족.

`st.current_items.is_empty()`로 "페이지/단에 처음 배치"를 판단하는 것이 더 명확할 수 있다.

---

## 3. 성능 저하

- `check_last_item_overflow`: `O(1)` 연산 (`current_items.last()`, 간단한 비교). 빈번 호출에도 overhead 무시 수준.
- `available_height()` 재계산: 덧셈/뺄셈 3–4회. 문제없음.
- carry 항목 높이 재산정: `mp.line_advances_sum(start_line..end_line)` 호출로 충분. 추가 measure 불필요.

**성능 관점에서 두 레이어 모두 문제없음.** 단, `advance_column_or_new_page`가 overflow 교정까지 담당하게 되면 단일 책임 원칙이 흐려져 이후 디버깅 난이도 상승.

---

## 4. 엣지 케이스

### `line_count == 1` — 레이어 1 실질 효과 없음

단일 줄 문단은 `cursor_line=0, end_line=1, end_line >= line_count`이므로
FullParagraph 경로로 처리되어 레이어 1 guard 대상(PartialParagraph)이 아니다.
기존 FullParagraph overflow 체크(`overflow_threshold > avail_for_lines`)에서만 방어된다.

### 빈 페이지 overflow — 안전

레이어 1: `!current_items.is_empty()` 가드로 advance 호출 안 함. 강제 배치.
레이어 2: `current_items.last()`가 None 반환. 아무것도 하지 않음. 안전.

### 다단 레이아웃 단 전환

레이어 2가 단 전환 시에도 carry를 발생시키므로, 위에서 지적한 carry 재삽입 누락 문제가 발생한다.

### `force_new_page` (쪽 나누기)

`check_last_item_overflow`를 거치지 않으므로 레이어 2 미작동. **이것은 올바른 동작이다.**
쪽 나누기는 overflow와 무관하게 무조건 새 페이지 이동이므로 carry를 삽입하면 의미가 훼손된다.

---

## 5. 전반적 개선 제안

### 우선순위 1 (구현 전 반드시 해결)

1. **레이어 2 carry 재삽입 시 `current_height` 보정** — 누락 시 이후 모든 배치 오류
2. **단 전환 경로 carry 재삽입 처리** — 누락 시 항목 소실

### 우선순위 2 (안정성)

3. **레이어 간 상호 배제** — 이중 이동/순서 역전 방지
4. **레이어 1 무한 루프 안전망** — `guard_advance_count` 상한 추가

### 우선순위 3 (완결성)

5. **`page_vpos_base` carry 재삽입 시 설정**
6. **레이어 1 `cursor_line == 0` 조건 재검토** — `current_items.is_empty()` 대비 평가

### 대안 설계 제안

초기 구현에서는 레이어 2를 `advance_column_or_new_page` 내부가 아닌 **문서 전체 배치 완료 후 검증** 방식으로 적용하는 것도 고려할 수 있다. while 루프 상태와 충돌하지 않아 훨씬 안전하지만, 수정 범위가 클 수 있다.
