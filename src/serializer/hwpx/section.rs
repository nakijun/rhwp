//! Contents/section{N}.xml — Section 본문 직렬화
//!
//! Stage 2.3: 다문단 + 소프트 라인브레이크 + 탭 (한컴 레퍼런스 ref_mixed.hwpx 기반)
//!
//! IR 매핑 관행:
//!   - `section.paragraphs` 여러 개 = 하드 문단 경계 (`<hp:p>` 여러 개)
//!   - `paragraph.text` 내 `\n` = 소프트 라인브레이크 (`<hp:lineBreak/>`, 같은 문단 내)
//!   - `paragraph.text` 내 `\t` = 탭 (`<hp:tab width=... leader="0" type="1"/>`)

use crate::model::document::{Document, Section};
use super::utils::xml_escape;
use super::SerializeError;

const EMPTY_SECTION_XML: &str = include_str!("templates/empty_section0.xml");
const TEXT_SLOT: &str = "<hp:t/>";
const LINESEG_SLOT_OPEN: &str = "<hp:linesegarray>";
const LINESEG_SLOT_CLOSE: &str = "</hp:linesegarray>";
const PARA_CLOSE: &str = "</hp:p></hs:sec>";

/// 레퍼런스 기준 줄 레이아웃 파라미터.
const VERT_STEP: u32 = 1600; // vertsize(1000) + spacing(600)
const LINE_FLAGS: u32 = 393216;
const HORZ_SIZE: u32 = 42520;
/// 탭 기본 폭 (한컴이 열면서 재계산하지만 초기값으로 필요).
const TAB_DEFAULT_WIDTH: u32 = 4000;

pub fn write_section(
    section: &Section,
    _doc: &Document,
    _index: usize,
) -> Result<Vec<u8>, SerializeError> {
    let mut vert_cursor: u32 = 0;

    // 첫 문단: 템플릿의 `<hp:t/>` 와 `<hp:linesegarray>` 영역을 치환.
    let first_text = section.paragraphs.first().map(|p| p.text.as_str()).unwrap_or("");
    let (first_t, first_linesegs, first_advance) = render_paragraph_parts(first_text, vert_cursor);
    vert_cursor = first_advance;

    let mut out = EMPTY_SECTION_XML.replacen(TEXT_SLOT, &first_t, 1);
    out = replace_first_linesegs(&out, &first_linesegs);

    // 추가 문단: `</hp:p></hs:sec>` 직전에 `<hp:p>` 요소를 삽입.
    if section.paragraphs.len() > 1 {
        let mut extra = String::new();
        for p in &section.paragraphs[1..] {
            let (t, linesegs, advance) = render_paragraph_parts(&p.text, vert_cursor);
            vert_cursor = advance;
            extra.push_str(r#"<hp:p id="0" paraPrIDRef="0" styleIDRef="0" pageBreak="0" columnBreak="0" merged="0"><hp:run charPrIDRef="0">"#);
            extra.push_str(&t);
            extra.push_str(r#"</hp:run><hp:linesegarray>"#);
            extra.push_str(&linesegs);
            extra.push_str(r#"</hp:linesegarray></hp:p>"#);
        }
        out = out.replacen(PARA_CLOSE, &format!("</hp:p>{}</hs:sec>", extra), 1);
    }

    Ok(out.into_bytes())
}

/// 문단 텍스트 하나를 (`<hp:t>` XML, lineseg XML, 다음 vert_cursor)로 변환.
fn render_paragraph_parts(text: &str, vert_start: u32) -> (String, String, u32) {
    let mut t_xml = String::from("<hp:t>");
    let mut linesegs = String::new();
    push_lineseg(&mut linesegs, 0, vert_start);

    let mut buf = String::new();
    let mut utf16_pos: u32 = 0;
    let mut lines_in_para: u32 = 0;

    for c in text.chars() {
        let u16_len = c.len_utf16() as u32;
        match c {
            '\t' => {
                flush_buf(&mut t_xml, &mut buf);
                t_xml.push_str(&format!(
                    r#"<hp:tab width="{}" leader="0" type="1"/>"#,
                    TAB_DEFAULT_WIDTH
                ));
                utf16_pos += u16_len;
            }
            '\n' => {
                flush_buf(&mut t_xml, &mut buf);
                t_xml.push_str("<hp:lineBreak/>");
                utf16_pos += u16_len;
                lines_in_para += 1;
                push_lineseg(
                    &mut linesegs,
                    utf16_pos,
                    vert_start + lines_in_para * VERT_STEP,
                );
            }
            c if (c as u32) < 0x20 => { /* 기타 제어문자 무시 */ }
            c => {
                buf.push(c);
                utf16_pos += u16_len;
            }
        }
    }
    flush_buf(&mut t_xml, &mut buf);
    t_xml.push_str("</hp:t>");

    // 이 문단이 차지한 줄 수 = 1 + 소프트 브레이크 수. 다음 문단 시작 vert 위치.
    let vert_end = vert_start + (lines_in_para + 1) * VERT_STEP;
    (t_xml, linesegs, vert_end)
}

fn flush_buf(t_xml: &mut String, buf: &mut String) {
    if !buf.is_empty() {
        t_xml.push_str(&xml_escape(buf));
        buf.clear();
    }
}

fn push_lineseg(out: &mut String, textpos: u32, vertpos: u32) {
    out.push_str(&format!(
        r#"<hp:lineseg textpos="{}" vertpos="{}" vertsize="1000" textheight="1000" baseline="850" spacing="600" horzpos="0" horzsize="{}" flags="{}"/>"#,
        textpos, vertpos, HORZ_SIZE, LINE_FLAGS,
    ));
}

fn replace_first_linesegs(xml: &str, new_inner: &str) -> String {
    let open = xml.find(LINESEG_SLOT_OPEN).expect("template has linesegarray");
    let inner_start = open + LINESEG_SLOT_OPEN.len();
    let close_rel = xml[inner_start..]
        .find(LINESEG_SLOT_CLOSE)
        .expect("template has closing linesegarray");
    let inner_end = inner_start + close_rel;
    let mut out = String::with_capacity(xml.len() + new_inner.len());
    out.push_str(&xml[..inner_start]);
    out.push_str(new_inner);
    out.push_str(&xml[inner_end..]);
    out
}
