//! Contents/section{N}.xml — Section 본문 직렬화
//!
//! Stage 2 (#182): 기존 템플릿 기반 구조를 유지하되, `<hp:p>` 와 `<hp:run>` 의 속성을
//! IR에서 가져와 동적으로 생성한다. `secPr`/`pagePr`/`grid` 등 섹션 정의는 템플릿 보존
//! (IR에 대응 필드가 더 담길 때까지 점진적으로 동적화 예정).
//!
//! IR 매핑 관행:
//!   - `section.paragraphs` 여러 개 = 하드 문단 경계 (`<hp:p>` 여러 개)
//!   - `paragraph.text` 내 `\n` = 소프트 라인브레이크 (`<hp:lineBreak/>`, 같은 문단 내)
//!   - `paragraph.text` 내 `\t` = 탭 (`<hp:tab width=... leader="0" type="1"/>`)
//!   - `paragraph.para_shape_id` → `<hp:p paraPrIDRef>`
//!   - `paragraph.style_id` → `<hp:p styleIDRef>`
//!   - `paragraph.column_type` → `<hp:p pageBreak/columnBreak>`
//!   - `paragraph.char_shapes[0].char_shape_id` → 첫 `<hp:run charPrIDRef>`

use crate::model::document::{Document, Section};
use crate::model::paragraph::{ColumnBreakType, Paragraph};

use super::context::SerializeContext;
use super::utils::xml_escape;
use super::SerializeError;

const EMPTY_SECTION_XML: &str = include_str!("templates/empty_section0.xml");
const TEXT_SLOT: &str = "<hp:t/>";
const LINESEG_SLOT_OPEN: &str = "<hp:linesegarray>";
const LINESEG_SLOT_CLOSE: &str = "</hp:linesegarray>";
const PARA_CLOSE: &str = "</hp:p></hs:sec>";

// 템플릿 내 첫 <hp:p> 태그의 실제 문자열 (id="3121190098" 랜덤 해시 포함).
// 템플릿은 정적이므로 이 문자열이 고정 위치에 있음이 보장됨.
const TEMPLATE_FIRST_P_TAG: &str =
    r#"<hp:p id="3121190098" paraPrIDRef="0" styleIDRef="0" pageBreak="0" columnBreak="0" merged="0">"#;
// 템플릿 내 <hp:run charPrIDRef="0"> 직후에 TEXT_SLOT 이 오는 패턴.
const TEMPLATE_RUN_BEFORE_TEXT: &str = r#"<hp:run charPrIDRef="0"><hp:t/>"#;

/// 레퍼런스 기준 줄 레이아웃 파라미터.
const VERT_STEP: u32 = 1600; // vertsize(1000) + spacing(600)
const LINE_FLAGS: u32 = 393216;
const HORZ_SIZE: u32 = 42520;
/// 탭 기본 폭 (한컴이 열면서 재계산하지만 초기값으로 필요).
const TAB_DEFAULT_WIDTH: u32 = 4000;

/// Stage 2 진입점. `ctx` 는 Stage 3+ 에서 파라미터 검증에 사용.
pub fn write_section(
    section: &Section,
    _doc: &Document,
    _index: usize,
    ctx: &SerializeContext,
) -> Result<Vec<u8>, SerializeError> {
    let _ = ctx;
    let mut vert_cursor: u32 = 0;

    let first_para = section.paragraphs.first();
    let first_text = first_para.map(|p| p.text.as_str()).unwrap_or("");
    let (first_t, first_linesegs, first_advance) = render_paragraph_parts(first_text, vert_cursor);
    vert_cursor = first_advance;

    let mut out = EMPTY_SECTION_XML.replacen(TEXT_SLOT, &first_t, 1);
    out = replace_first_linesegs(&out, &first_linesegs);

    // 첫 문단 `<hp:p>` 태그를 IR 기반 속성으로 교체
    if let Some(p) = first_para {
        let new_p_tag = render_hp_p_open(p, 0);
        out = out.replacen(TEMPLATE_FIRST_P_TAG, &new_p_tag, 1);

        // 첫 문단의 텍스트용 <hp:run> 의 charPrIDRef 를 IR 기반으로 교체
        // 템플릿에서 TEXT_SLOT 이 있던 자리 바로 앞의 <hp:run charPrIDRef="0"> 패턴.
        let first_run_cs = first_run_char_shape_id(p);
        let new_run = format!(r#"<hp:run charPrIDRef="{}">"#, first_run_cs);
        let replacement = format!("{}{}", new_run, &first_t);
        // 이미 first_t 는 out 에 들어갔으므로 그 직전의 <hp:run charPrIDRef="0"> 만 변경
        let anchor = format!("{}{}", r#"<hp:run charPrIDRef="0">"#, &first_t);
        if out.contains(&anchor) {
            out = out.replacen(&anchor, &replacement, 1);
        }
    }

    // 추가 문단: `</hp:p></hs:sec>` 직전에 `<hp:p>` 요소를 삽입.
    if section.paragraphs.len() > 1 {
        let mut extra = String::new();
        for (idx, p) in section.paragraphs.iter().enumerate().skip(1) {
            let (t, linesegs, advance) = render_paragraph_parts(&p.text, vert_cursor);
            vert_cursor = advance;
            let cs = first_run_char_shape_id(p);
            extra.push_str(&render_hp_p_open(p, idx as u32));
            extra.push_str(&format!(r#"<hp:run charPrIDRef="{}">"#, cs));
            extra.push_str(&t);
            extra.push_str(r#"</hp:run><hp:linesegarray>"#);
            extra.push_str(&linesegs);
            extra.push_str(r#"</hp:linesegarray></hp:p>"#);
        }
        out = out.replacen(PARA_CLOSE, &format!("</hp:p>{}</hs:sec>", extra), 1);
    }

    Ok(out.into_bytes())
}

/// IR의 Paragraph를 기반으로 `<hp:p>` 시작 태그를 생성.
///
/// `id` 는 문단 순서 기반(0, 1, 2, ...)로 할당한다. 한컴 샘플은 랜덤 해시도 쓰지만
/// 파서는 id 를 무시하므로 순차값으로 충분.
fn render_hp_p_open(p: &Paragraph, id: u32) -> String {
    let page_break = if matches!(p.column_type, ColumnBreakType::Page) { 1 } else { 0 };
    let column_break = if matches!(p.column_type, ColumnBreakType::Column) { 1 } else { 0 };
    format!(
        r#"<hp:p id="{}" paraPrIDRef="{}" styleIDRef="{}" pageBreak="{}" columnBreak="{}" merged="0">"#,
        id, p.para_shape_id, p.style_id, page_break, column_break,
    )
}

/// 문단 첫 run 의 charPrIDRef. IR의 `char_shapes[0].char_shape_id` 사용.
/// 비어있으면 0 (기본 글자모양) 반환.
fn first_run_char_shape_id(p: &Paragraph) -> u32 {
    p.char_shapes.first().map(|r| r.char_shape_id).unwrap_or(0)
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

// `TEMPLATE_RUN_BEFORE_TEXT` 는 패턴 인식용 상수로만 쓰이므로 명시 참조.
#[allow(dead_code)]
fn _template_anchor_hint() {
    let _ = TEMPLATE_RUN_BEFORE_TEXT;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::paragraph::{CharShapeRef, Paragraph};

    fn make_doc_with_paragraph(para: Paragraph) -> (Document, Section) {
        let mut section = Section::default();
        section.paragraphs.push(para);
        let mut doc = Document::default();
        doc.sections.push(section.clone());
        (doc, section)
    }

    #[test]
    fn hp_p_attrs_reflect_para_shape_id_and_style_id() {
        let mut para = Paragraph::default();
        para.para_shape_id = 7;
        para.style_id = 3;
        para.text = "hi".to_string();
        let (doc, section) = make_doc_with_paragraph(para);
        let ctx = SerializeContext::collect_from_document(&doc);
        let bytes = write_section(&section, &doc, 0, &ctx).unwrap();
        let xml = std::str::from_utf8(&bytes).unwrap();
        assert!(
            xml.contains(r#"paraPrIDRef="7""#),
            "<hp:p> must reflect para_shape_id=7: {}",
            &xml[..200.min(xml.len())]
        );
        assert!(
            xml.contains(r#"styleIDRef="3""#),
            "<hp:p> must reflect style_id=3"
        );
    }

    #[test]
    fn hp_run_reflects_first_char_shape_id() {
        let mut para = Paragraph::default();
        para.text = "hello".to_string();
        para.char_shapes.push(CharShapeRef {
            start_pos: 0,
            char_shape_id: 42,
        });
        let (doc, section) = make_doc_with_paragraph(para);
        let ctx = SerializeContext::collect_from_document(&doc);
        let bytes = write_section(&section, &doc, 0, &ctx).unwrap();
        let xml = std::str::from_utf8(&bytes).unwrap();
        assert!(
            xml.contains(r#"<hp:run charPrIDRef="42"><hp:t>hello</hp:t>"#),
            "first run must use char_shape_id 42, xml excerpt around <hp:t>: {:?}",
            xml.find("<hp:t>").map(|i| &xml[i.saturating_sub(50)..(i + 50).min(xml.len())])
        );
    }

    #[test]
    fn page_break_paragraph_emits_attr() {
        let mut para = Paragraph::default();
        para.text = "p1".to_string();
        para.column_type = crate::model::paragraph::ColumnBreakType::Page;
        let (doc, section) = make_doc_with_paragraph(para);
        let ctx = SerializeContext::collect_from_document(&doc);
        let bytes = write_section(&section, &doc, 0, &ctx).unwrap();
        let xml = std::str::from_utf8(&bytes).unwrap();
        assert!(
            xml.contains(r#"pageBreak="1""#),
            "pageBreak must be 1 for Page column_type"
        );
        assert!(xml.contains(r#"columnBreak="0""#));
    }

    #[test]
    fn default_paragraph_keeps_zero_attrs() {
        let mut para = Paragraph::default();
        para.text = "x".to_string();
        let (doc, section) = make_doc_with_paragraph(para);
        let ctx = SerializeContext::collect_from_document(&doc);
        let bytes = write_section(&section, &doc, 0, &ctx).unwrap();
        let xml = std::str::from_utf8(&bytes).unwrap();
        assert!(xml.contains(r#"paraPrIDRef="0""#));
        assert!(xml.contains(r#"styleIDRef="0""#));
        // char_shapes 가 비어있으면 fallback 0
        assert!(xml.contains(r#"<hp:run charPrIDRef="0">"#));
    }

    #[test]
    fn additional_paragraphs_use_their_own_char_shape() {
        let mut p1 = Paragraph::default();
        p1.text = "first".to_string();
        p1.char_shapes.push(CharShapeRef { start_pos: 0, char_shape_id: 5 });
        let mut p2 = Paragraph::default();
        p2.text = "second".to_string();
        p2.para_shape_id = 2;
        p2.char_shapes.push(CharShapeRef { start_pos: 0, char_shape_id: 6 });
        let mut section = Section::default();
        section.paragraphs.push(p1);
        section.paragraphs.push(p2);
        let mut doc = Document::default();
        doc.sections.push(section.clone());
        let ctx = SerializeContext::collect_from_document(&doc);
        let xml = String::from_utf8(write_section(&section, &doc, 0, &ctx).unwrap()).unwrap();
        // 두 번째 문단: paraPrIDRef=2, charPrIDRef=6
        assert!(xml.contains(r#"paraPrIDRef="2""#));
        assert!(
            xml.matches(r#"charPrIDRef="6""#).count() >= 1,
            "second paragraph must emit charPrIDRef=6"
        );
    }
}
