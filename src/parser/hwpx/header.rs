//! header.xml 파싱 — HWPX 문서 메타데이터를 DocInfo로 변환
//!
//! header.xml은 글꼴, 글자모양, 문단모양, 스타일, 테두리/배경 등
//! 문서 전체에서 참조하는 리소스 테이블을 포함한다.

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::model::document::{DocInfo, DocProperties};
use crate::model::style::*;

use super::HwpxError;
use super::utils::{local_name, attr_str, parse_u8, parse_i8, parse_u16, parse_i16, parse_u32, parse_i32, parse_color, parse_bool};

/// header.xml을 파싱하여 DocInfo와 DocProperties를 생성한다.
pub fn parse_hwpx_header(xml: &str) -> Result<(DocInfo, DocProperties), HwpxError> {
    let mut doc_info = DocInfo::default();
    let mut doc_props = DocProperties::default();

    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();

    // 기본값: 7개 언어별 빈 글꼴 목록
    doc_info.font_faces = vec![Vec::new(); 7];

    // 현재 <fontface lang="..."> 컨텍스트 추적
    // HANGUL=0, LATIN=1, HANJA=2, JAPANESE=3, OTHER=4, SYMBOL=5, USER=6
    let mut current_font_group: usize = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name(); let local = local_name(name.as_ref());
                match local {
                    b"fontface" => {
                        // <hh:fontface lang="HANGUL"> → 언어 그룹 설정
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"lang" {
                                current_font_group = match attr_str(&attr).as_str() {
                                    "HANGUL" => 0,
                                    "LATIN" => 1,
                                    "HANJA" => 2,
                                    "JAPANESE" => 3,
                                    "OTHER" => 4,
                                    "SYMBOL" => 5,
                                    "USER" => 6,
                                    _ => 0,
                                };
                            }
                        }
                    }
                    b"beginNum" => parse_begin_num(e, &mut doc_props),
                    b"font" => parse_font(e, &mut doc_info, current_font_group),
                    b"charPr" => {
                        parse_char_shape(e, &mut reader, &mut doc_info)?;
                    }
                    b"paraPr" => {
                        parse_para_shape(e, &mut reader, &mut doc_info)?;
                    }
                    b"style" => parse_style(e, &mut doc_info),
                    b"borderFill" => {
                        parse_border_fill(e, &mut reader, &mut doc_info)?;
                    }
                    b"tabPr" => {
                        parse_tab_def(e, &mut reader, &mut doc_info)?;
                    }
                    b"numbering" => {
                        parse_numbering(e, &mut reader, &mut doc_info)?;
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let name = e.name(); let local = local_name(name.as_ref());
                match local {
                    b"beginNum" => parse_begin_num(e, &mut doc_props),
                    b"font" => parse_font(e, &mut doc_info, current_font_group),
                    b"style" => parse_style(e, &mut doc_info),
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(HwpxError::XmlError(format!("header.xml: {}", e))),
            _ => {}
        }
        buf.clear();
    }

    doc_props.section_count = 1; // content.hpf에서 갱신됨

    Ok((doc_info, doc_props))
}

// ─── beginNum ───

fn parse_begin_num(e: &quick_xml::events::BytesStart, props: &mut DocProperties) {
    for attr in e.attributes().flatten() {
        match attr.key.as_ref() {
            b"page" => props.page_start_num = parse_u16(&attr),
            b"footnote" => props.footnote_start_num = parse_u16(&attr),
            b"endnote" => props.endnote_start_num = parse_u16(&attr),
            b"pic" => props.picture_start_num = parse_u16(&attr),
            b"tbl" => props.table_start_num = parse_u16(&attr),
            b"equation" => props.equation_start_num = parse_u16(&attr),
            _ => {}
        }
    }
}

// ─── Font ───

fn parse_font(e: &quick_xml::events::BytesStart, doc_info: &mut DocInfo, font_group: usize) {
    let mut name = String::new();

    for attr in e.attributes().flatten() {
        match attr.key.as_ref() {
            b"face" => name = attr_str(&attr),
            _ => {}
        }
    }

    if !name.is_empty() {
        let font = Font {
            name,
            ..Default::default()
        };
        // fontface lang 컨텍스트에 따라 해당 언어 그룹에 추가
        if font_group < doc_info.font_faces.len() {
            doc_info.font_faces[font_group].push(font);
        }
    }
}

// ─── CharShape ───

fn parse_char_shape(
    e: &quick_xml::events::BytesStart,
    reader: &mut Reader<&[u8]>,
    doc_info: &mut DocInfo,
) -> Result<(), HwpxError> {
    let mut cs = CharShape::default();

    for attr in e.attributes().flatten() {
        match attr.key.as_ref() {
            b"height" => cs.base_size = parse_i32(&attr),
            b"textColor" => cs.text_color = parse_color(&attr),
            b"shadeColor" => cs.shade_color = parse_color(&attr),
            b"useFontSpace" | b"useKerning" | b"symMark" => {}
            b"borderFillIDRef" => cs.border_fill_id = parse_u16(&attr),
            _ => {}
        }
    }

    // 자식 요소 파싱
    if !is_empty_event(e) {
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref ce)) | Ok(Event::Start(ref ce)) => {
                    let cname = ce.name(); let local = local_name(cname.as_ref());
                    match local {
                        b"fontRef" => {
                            for attr in ce.attributes().flatten() {
                                let val = parse_u16(&attr);
                                match attr.key.as_ref() {
                                    b"hangul" => cs.font_ids[0] = val,
                                    b"latin" => cs.font_ids[1] = val,
                                    b"hanja" => cs.font_ids[2] = val,
                                    b"japanese" => cs.font_ids[3] = val,
                                    b"other" => cs.font_ids[4] = val,
                                    b"symbol" => cs.font_ids[5] = val,
                                    b"user" => cs.font_ids[6] = val,
                                    _ => {}
                                }
                            }
                        }
                        b"ratio" => {
                            for attr in ce.attributes().flatten() {
                                let val = parse_u8(&attr);
                                match attr.key.as_ref() {
                                    b"hangul" => cs.ratios[0] = val,
                                    b"latin" => cs.ratios[1] = val,
                                    b"hanja" => cs.ratios[2] = val,
                                    b"japanese" => cs.ratios[3] = val,
                                    b"other" => cs.ratios[4] = val,
                                    b"symbol" => cs.ratios[5] = val,
                                    b"user" => cs.ratios[6] = val,
                                    _ => {}
                                }
                            }
                        }
                        b"spacing" => {
                            for attr in ce.attributes().flatten() {
                                let val = parse_i8(&attr);
                                match attr.key.as_ref() {
                                    b"hangul" => cs.spacings[0] = val,
                                    b"latin" => cs.spacings[1] = val,
                                    b"hanja" => cs.spacings[2] = val,
                                    b"japanese" => cs.spacings[3] = val,
                                    b"other" => cs.spacings[4] = val,
                                    b"symbol" => cs.spacings[5] = val,
                                    b"user" => cs.spacings[6] = val,
                                    _ => {}
                                }
                            }
                        }
                        b"relSz" => {
                            for attr in ce.attributes().flatten() {
                                let val = parse_u8(&attr);
                                match attr.key.as_ref() {
                                    b"hangul" => cs.relative_sizes[0] = val,
                                    b"latin" => cs.relative_sizes[1] = val,
                                    b"hanja" => cs.relative_sizes[2] = val,
                                    b"japanese" => cs.relative_sizes[3] = val,
                                    b"other" => cs.relative_sizes[4] = val,
                                    b"symbol" => cs.relative_sizes[5] = val,
                                    b"user" => cs.relative_sizes[6] = val,
                                    _ => {}
                                }
                            }
                        }
                        b"offset" => {
                            for attr in ce.attributes().flatten() {
                                let val = parse_i8(&attr);
                                match attr.key.as_ref() {
                                    b"hangul" => cs.char_offsets[0] = val,
                                    b"latin" => cs.char_offsets[1] = val,
                                    b"hanja" => cs.char_offsets[2] = val,
                                    b"japanese" => cs.char_offsets[3] = val,
                                    b"other" => cs.char_offsets[4] = val,
                                    b"symbol" => cs.char_offsets[5] = val,
                                    b"user" => cs.char_offsets[6] = val,
                                    _ => {}
                                }
                            }
                        }
                        b"bold" => cs.bold = true,
                        b"italic" => cs.italic = true,
                        b"underline" => {
                            for attr in ce.attributes().flatten() {
                                if attr.key.as_ref() == b"type" {
                                    let val = attr_str(&attr);
                                    cs.underline_type = match val.as_str() {
                                        "BOTTOM" => UnderlineType::Bottom,
                                        "TOP" => UnderlineType::Top,
                                        _ => UnderlineType::None,
                                    };
                                }
                                if attr.key.as_ref() == b"color" {
                                    cs.underline_color = parse_color(&attr);
                                }
                            }
                        }
                        b"strikeout" => {
                            for attr in ce.attributes().flatten() {
                                if attr.key.as_ref() == b"shape" {
                                    let val = attr_str(&attr);
                                    // 유효한 취소선: SOLID, DASH, DOT 등 선 스타일
                                    // NONE, 3D 등은 취소선 없음으로 처리
                                    cs.strikethrough = matches!(
                                        val.as_str(),
                                        "SOLID" | "DASH" | "DOT" | "DASH_DOT" | "DASH_DOT_DOT"
                                        | "LONG_DASH" | "CIRCLE" | "DOUBLE_SLIM"
                                        | "SLIM_THICK" | "THICK_SLIM" | "SLIM_THICK_SLIM"
                                    );
                                }
                                if attr.key.as_ref() == b"color" {
                                    cs.strike_color = parse_color(&attr);
                                }
                            }
                        }
                        b"outline" => {
                            for attr in ce.attributes().flatten() {
                                if attr.key.as_ref() == b"type" {
                                    let val = attr_str(&attr);
                                    cs.outline_type = match val.as_str() {
                                        "NONE" => 0,
                                        "SOLID" => 1,
                                        "DASH" => 2,
                                        "DOT" => 3,
                                        _ => 0,
                                    };
                                }
                            }
                        }
                        b"shadow" => {
                            for attr in ce.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"type" => {
                                        let val = attr_str(&attr);
                                        cs.shadow_type = match val.as_str() {
                                            "NONE" => 0,
                                            "DROP" | "CONTINUOUS" => 1,
                                            _ => 0,
                                        };
                                    }
                                    b"color" => cs.shadow_color = parse_color(&attr),
                                    _ => {}
                                }
                            }
                        }
                        b"emboss" => { cs.attr |= 1 << 13; cs.emboss = true; }
                        b"engrave" => { cs.attr |= 1 << 14; cs.engrave = true; }
                        b"supscript" => cs.superscript = true,
                        b"subscript" => cs.subscript = true,
                        _ => {}
                    }
                }
                Ok(Event::End(ref ee)) => {
                    let ename = ee.name(); if local_name(ename.as_ref()) == b"charPr" {
                        break;
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(HwpxError::XmlError(format!("charPr: {}", e))),
                _ => {}
            }
            buf.clear();
        }
    }

    doc_info.char_shapes.push(cs);
    Ok(())
}

// ─── ParaShape ───

fn parse_para_shape(
    e: &quick_xml::events::BytesStart,
    reader: &mut Reader<&[u8]>,
    doc_info: &mut DocInfo,
) -> Result<(), HwpxError> {
    let mut ps = ParaShape::default();

    for attr in e.attributes().flatten() {
        match attr.key.as_ref() {
            b"tabPrIDRef" => ps.tab_def_id = parse_u16(&attr),
            b"condense" => {}
            _ => {}
        }
    }

    if !is_empty_event(e) {
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref ce)) | Ok(Event::Start(ref ce)) => {
                    let cname = ce.name(); let local = local_name(cname.as_ref());
                    match local {
                        b"align" => {
                            for attr in ce.attributes().flatten() {
                                if attr.key.as_ref() == b"horizontal" {
                                    ps.alignment = parse_alignment(&attr);
                                }
                            }
                        }
                        b"heading" => {
                            for attr in ce.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"type" => {
                                        let val = attr_str(&attr);
                                        ps.head_type = match val.as_str() {
                                            "OUTLINE" => HeadType::Outline,
                                            "NUMBER" | "NUMBERING" => HeadType::Number,
                                            "BULLET" => HeadType::Bullet,
                                            _ => HeadType::None,
                                        };
                                    }
                                    b"idRef" => ps.numbering_id = parse_u16(&attr),
                                    b"level" => ps.para_level = parse_u8(&attr),
                                    _ => {}
                                }
                            }
                        }
                        b"margin" => {
                            for attr in ce.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"left" => ps.margin_left = parse_i32(&attr),
                                    b"right" => ps.margin_right = parse_i32(&attr),
                                    b"indent" => ps.indent = parse_i32(&attr),
                                    b"prev" => ps.spacing_before = parse_i32(&attr),
                                    b"next" => ps.spacing_after = parse_i32(&attr),
                                    _ => {}
                                }
                            }
                        }
                        b"lineSpacing" => {
                            for attr in ce.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"type" => {
                                        let val = attr_str(&attr);
                                        ps.line_spacing_type = match val.as_str() {
                                            "PERCENT" => LineSpacingType::Percent,
                                            "FIXED" => LineSpacingType::Fixed,
                                            "SPACEONLY" | "SPACE_ONLY" => LineSpacingType::SpaceOnly,
                                            "MINIMUM" | "AT_LEAST" => LineSpacingType::Minimum,
                                            _ => LineSpacingType::Percent,
                                        };
                                    }
                                    b"value" => ps.line_spacing = parse_i32(&attr),
                                    _ => {}
                                }
                            }
                        }
                        b"border" => {
                            for attr in ce.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"borderFillIDRef" => ps.border_fill_id = parse_u16(&attr),
                                    b"offsetLeft" => ps.border_spacing[0] = parse_i16(&attr),
                                    b"offsetRight" => ps.border_spacing[1] = parse_i16(&attr),
                                    b"offsetTop" => ps.border_spacing[2] = parse_i16(&attr),
                                    b"offsetBottom" => ps.border_spacing[3] = parse_i16(&attr),
                                    _ => {}
                                }
                            }
                        }
                        b"breakSetting" => {
                            for attr in ce.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"widowOrphan" => if parse_bool(&attr) {
                                        ps.attr2 |= 1 << 5;
                                    },
                                    b"keepWithNext" => if parse_bool(&attr) {
                                        ps.attr2 |= 1 << 6;
                                    },
                                    b"keepLines" => if parse_bool(&attr) {
                                        ps.attr2 |= 1 << 7;
                                    },
                                    b"pageBreakBefore" => if parse_bool(&attr) {
                                        ps.attr2 |= 1 << 8;
                                    },
                                    _ => {}
                                }
                            }
                        }
                        b"autoSpacing" => {
                            for attr in ce.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"eAsianEng" => if parse_bool(&attr) {
                                        ps.attr1 |= 1 << 20;
                                    },
                                    b"eAsianNum" => if parse_bool(&attr) {
                                        ps.attr1 |= 1 << 21;
                                    },
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref ee)) => {
                    let ename = ee.name(); if local_name(ename.as_ref()) == b"paraPr" {
                        break;
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(HwpxError::XmlError(format!("paraPr: {}", e))),
                _ => {}
            }
            buf.clear();
        }
    }

    doc_info.para_shapes.push(ps);
    Ok(())
}

// ─── Style ───

fn parse_style(e: &quick_xml::events::BytesStart, doc_info: &mut DocInfo) {
    let mut style = Style::default();
    for attr in e.attributes().flatten() {
        match attr.key.as_ref() {
            b"name" => style.local_name = attr_str(&attr),
            b"engName" => style.english_name = attr_str(&attr),
            b"type" => {
                let val = attr_str(&attr);
                style.style_type = match val.as_str() {
                    "PARA" | "PARAGRAPH" => 0,
                    "CHAR" | "CHARACTER" => 1,
                    _ => 0,
                };
            }
            b"paraPrIDRef" => style.para_shape_id = parse_u16(&attr),
            b"charPrIDRef" => style.char_shape_id = parse_u16(&attr),
            b"nextStyleIDRef" => style.next_style_id = parse_u8(&attr),
            _ => {}
        }
    }
    doc_info.styles.push(style);
}

// ─── BorderFill ───

fn parse_border_fill(
    e: &quick_xml::events::BytesStart,
    reader: &mut Reader<&[u8]>,
    doc_info: &mut DocInfo,
) -> Result<(), HwpxError> {
    let mut bf = BorderFill::default();

    if !is_empty_event(e) {
        let mut buf = Vec::new();
        let mut border_idx = 0usize;
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref ce)) | Ok(Event::Start(ref ce)) => {
                    let cname = ce.name(); let local = local_name(cname.as_ref());
                    match local {
                        b"leftBorder" | b"rightBorder" | b"topBorder" | b"bottomBorder" => {
                            let idx = match local {
                                b"leftBorder" => 0,
                                b"rightBorder" => 1,
                                b"topBorder" => 2,
                                b"bottomBorder" => 3,
                                _ => { border_idx += 1; border_idx - 1 }
                            };
                            if idx < 4 {
                                for attr in ce.attributes().flatten() {
                                    match attr.key.as_ref() {
                                        b"type" => bf.borders[idx].line_type = parse_border_line_type(&attr),
                                        b"width" => bf.borders[idx].width = parse_border_width(&attr),
                                        b"color" => bf.borders[idx].color = parse_color(&attr),
                                        _ => {}
                                    }
                                }
                            }
                        }
                        b"diagonal" => {
                            for attr in ce.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"type" => bf.diagonal.diagonal_type = parse_u8(&attr),
                                    b"width" => bf.diagonal.width = parse_border_width(&attr),
                                    b"color" => bf.diagonal.color = parse_color(&attr),
                                    _ => {}
                                }
                            }
                        }
                        b"fillBrush" => {
                            // fillBrush 자식 요소를 파싱
                            // Start 이벤트이면 자식을 읽어야 함
                        }
                        b"winBrush" => {
                            bf.fill.fill_type = FillType::Solid;
                            let mut solid = SolidFill::default();
                            for attr in ce.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"faceColor" => solid.background_color = parse_color(&attr),
                                    b"hatchColor" => solid.pattern_color = parse_color(&attr),
                                    b"alpha" => {
                                        // HWPX alpha: 0.0=완전투명 ~ 1.0=불투명 (float string)
                                        let val = attr_str(&attr);
                                        if let Ok(f) = val.parse::<f64>() {
                                            bf.fill.alpha = (f.clamp(0.0, 1.0) * 255.0) as u8;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            bf.fill.solid = Some(solid);
                        }
                        b"gradation" => {
                            bf.fill.fill_type = FillType::Gradient;
                            let mut grad = GradientFill::default();
                            for attr in ce.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"type" => grad.gradient_type = parse_i16(&attr),
                                    b"angle" => grad.angle = parse_i16(&attr),
                                    b"centerX" => grad.center_x = parse_i16(&attr),
                                    b"centerY" => grad.center_y = parse_i16(&attr),
                                    b"blur" => grad.blur = parse_i16(&attr),
                                    _ => {}
                                }
                            }
                            bf.fill.gradient = Some(grad);
                        }
                        b"color" => {
                            // <hh:color value="#RRGGBB"/> — gradation 자식
                            if let Some(ref mut grad) = bf.fill.gradient {
                                for attr in ce.attributes().flatten() {
                                    if attr.key.as_ref() == b"value" {
                                        grad.colors.push(parse_color(&attr));
                                    }
                                }
                            }
                        }
                        b"imgBrush" => {
                            bf.fill.fill_type = FillType::Image;
                            let mut img_fill = ImageFill::default();
                            for attr in ce.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"mode" => {
                                        img_fill.fill_mode = match attr_str(&attr).as_str() {
                                            "TILE" | "TILE_ALL" => ImageFillMode::TileAll,
                                            "TILE_HORZ_TOP" => ImageFillMode::TileHorzTop,
                                            "TILE_HORZ_BOTTOM" => ImageFillMode::TileHorzBottom,
                                            "TILE_VERT_LEFT" => ImageFillMode::TileVertLeft,
                                            "TILE_VERT_RIGHT" => ImageFillMode::TileVertRight,
                                            "CENTER" => ImageFillMode::Center,
                                            "CENTER_TOP" => ImageFillMode::CenterTop,
                                            "CENTER_BOTTOM" => ImageFillMode::CenterBottom,
                                            "FIT" | "FIT_TO_SIZE" | "STRETCH" | "TOTAL" => ImageFillMode::FitToSize,
                                            "TOP_LEFT_ALIGN" => ImageFillMode::LeftTop,
                                            _ => ImageFillMode::TileAll,
                                        };
                                    }
                                    b"bright" => img_fill.brightness = parse_i8(&attr),
                                    b"contrast" => img_fill.contrast = parse_i8(&attr),
                                    _ => {}
                                }
                            }
                            bf.fill.image = Some(img_fill);
                        }
                        b"img" | b"image" => {
                            // imgBrush 내부의 이미지 참조
                            if let Some(ref mut img_fill) = bf.fill.image {
                                for attr in ce.attributes().flatten() {
                                    if attr.key.as_ref() == b"binaryItemIDRef" {
                                        let val = attr_str(&attr);
                                        let num: String = val.chars().filter(|c| c.is_ascii_digit()).collect();
                                        img_fill.bin_data_id = num.parse().unwrap_or(0);
                                    }
                                }
                            }
                        }
                        b"slash" => {
                            for attr in ce.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"type" => bf.diagonal.diagonal_type = parse_u8(&attr),
                                    b"width" => bf.diagonal.width = parse_border_width(&attr),
                                    b"color" => bf.diagonal.color = parse_color(&attr),
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref ee)) => {
                    let ename = ee.name(); if local_name(ename.as_ref()) == b"borderFill" {
                        break;
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(HwpxError::XmlError(format!("borderFill: {}", e))),
                _ => {}
            }
            buf.clear();
        }
    }

    doc_info.border_fills.push(bf);
    Ok(())
}

// ─── TabDef ───

fn parse_tab_def(
    e: &quick_xml::events::BytesStart,
    reader: &mut Reader<&[u8]>,
    doc_info: &mut DocInfo,
) -> Result<(), HwpxError> {
    let mut td = TabDef::default();

    for attr in e.attributes().flatten() {
        match attr.key.as_ref() {
            b"autoTabLeft" => td.auto_tab_left = attr_str(&attr) == "1",
            b"autoTabRight" => td.auto_tab_right = attr_str(&attr) == "1",
            _ => {}
        }
    }

    if !is_empty_event(e) {
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref ce)) | Ok(Event::Start(ref ce)) => {
                    let cname = ce.name(); let local = local_name(cname.as_ref());
                    if local == b"tabItem" {
                        let mut item = TabItem::default();
                        for attr in ce.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"pos" => item.position = parse_u32(&attr),
                                b"type" => {
                                    let val = attr_str(&attr);
                                    item.tab_type = match val.as_str() {
                                        "LEFT" => 0,
                                        "RIGHT" => 1,
                                        "CENTER" => 2,
                                        "DECIMAL" => 3,
                                        _ => 0,
                                    };
                                }
                                b"leader" => {
                                    let val = attr_str(&attr);
                                    item.fill_type = match val.as_str() {
                                        "NONE" => 0,
                                        "SOLID" => 1,
                                        "DASH" => 2,
                                        "DOT" => 3,
                                        _ => 0,
                                    };
                                }
                                _ => {}
                            }
                        }
                        td.tabs.push(item);
                    }
                }
                Ok(Event::End(ref ee)) => {
                    let ename = ee.name(); if local_name(ename.as_ref()) == b"tabPr" {
                        break;
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(HwpxError::XmlError(format!("tabPr: {}", e))),
                _ => {}
            }
            buf.clear();
        }
    }

    doc_info.tab_defs.push(td);
    Ok(())
}

// ─── Numbering ───

fn parse_numbering(
    e: &quick_xml::events::BytesStart,
    reader: &mut Reader<&[u8]>,
    doc_info: &mut DocInfo,
) -> Result<(), HwpxError> {
    let mut num = Numbering::default();

    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == b"start" {
            num.start_number = parse_u16(&attr);
        }
    }

    if !is_empty_event(e) {
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref ce)) | Ok(Event::Start(ref ce)) => {
                    let cname = ce.name(); let local = local_name(cname.as_ref());
                    if local == b"paraHead" {
                        let mut level: usize = 0;
                        let mut head = NumberingHead::default();
                        let mut format_str = String::new();
                        for attr in ce.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"level" => level = parse_u32(&attr) as usize,
                                b"start" => {
                                    let s = parse_u32(&attr);
                                    if level > 0 && level <= 7 {
                                        num.level_start_numbers[level - 1] = s;
                                    }
                                }
                                b"text" => format_str = attr_str(&attr),
                                b"numFormat" => head.number_format = parse_u8(&attr),
                                b"charPrIDRef" => head.char_shape_id = parse_u32(&attr),
                                _ => {}
                            }
                        }
                        if level > 0 && level <= 7 {
                            num.heads[level - 1] = head;
                            num.level_formats[level - 1] = format_str;
                        }
                    }
                }
                Ok(Event::End(ref ee)) => {
                    let ename = ee.name(); if local_name(ename.as_ref()) == b"numbering" {
                        break;
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(HwpxError::XmlError(format!("numbering: {}", e))),
                _ => {}
            }
            buf.clear();
        }
    }

    doc_info.numberings.push(num);
    Ok(())
}

// ─── 유틸리티 함수 (header 전용) ───

fn is_empty_event(_e: &quick_xml::events::BytesStart) -> bool {
    // quick-xml의 Event::Empty vs Event::Start 구분으로 판단
    // 호출측에서 Empty/Start 구분 없이 패턴 매칭하므로 항상 false 반환
    // (자식 파싱 루프가 End 태그에서 break하므로 안전)
    false
}

fn parse_alignment(attr: &quick_xml::events::attributes::Attribute) -> Alignment {
    match attr_str(attr).as_str() {
        "JUSTIFY" => Alignment::Justify,
        "LEFT" => Alignment::Left,
        "RIGHT" => Alignment::Right,
        "CENTER" => Alignment::Center,
        "DISTRIBUTE" => Alignment::Distribute,
        _ => Alignment::Justify,
    }
}

fn parse_border_line_type(attr: &quick_xml::events::attributes::Attribute) -> BorderLineType {
    match attr_str(attr).as_str() {
        "NONE" => BorderLineType::None,
        "SOLID" => BorderLineType::Solid,
        "DASH" => BorderLineType::Dash,
        "DOT" => BorderLineType::Dot,
        "DASH_DOT" => BorderLineType::DashDot,
        "DASH_DOT_DOT" => BorderLineType::DashDotDot,
        "LONG_DASH" => BorderLineType::LongDash,
        "CIRCLE" => BorderLineType::Circle,
        "DOUBLE_SLIM" | "DOUBLE" => BorderLineType::Double,
        "SLIM_THICK" => BorderLineType::ThinThickDouble,
        "THICK_SLIM" => BorderLineType::ThickThinDouble,
        "SLIM_THICK_SLIM" => BorderLineType::ThinThickThinTriple,
        "WAVE" => BorderLineType::Wave,
        "DOUBLE_WAVE" => BorderLineType::DoubleWave,
        _ => BorderLineType::Solid,
    }
}

fn parse_border_width(attr: &quick_xml::events::attributes::Attribute) -> u8 {
    let s = attr_str(attr);
    // "0.12 mm", "0.4 mm" 등의 형식에서 두께 인덱스 추출
    let mm: f64 = s.split_whitespace()
        .next()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.12);
    // 대략적인 HWP 두께 인덱스 매핑
    if mm <= 0.12 { 0 }
    else if mm <= 0.3 { 1 }
    else if mm <= 0.5 { 2 }
    else if mm <= 1.0 { 3 }
    else if mm <= 1.5 { 4 }
    else { 5 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_color_rgb() {
        let attr_data = b"#FF0000";
        // 빨강: RRGGBB → 0x000000FF (BBGGRR)
        let xml = r##"<e color="#FF0000"/>"##.to_string();
        let mut reader = Reader::from_str(&xml);
        let mut buf = Vec::new();
        if let Ok(Event::Empty(ref e)) = reader.read_event_into(&mut buf) {
            for attr in e.attributes().flatten() {
                if attr.key.as_ref() == b"color" {
                    assert_eq!(parse_color(&attr), 0x000000FF);
                }
            }
        }
    }

    #[test]
    fn test_parse_color_none() {
        let xml = r#"<e color="none"/>"#;
        let mut reader = Reader::from_str(xml);
        let mut buf = Vec::new();
        if let Ok(Event::Empty(ref e)) = reader.read_event_into(&mut buf) {
            for attr in e.attributes().flatten() {
                if attr.key.as_ref() == b"color" {
                    assert_eq!(parse_color(&attr), 0xFFFFFFFF);
                }
            }
        }
    }

    #[test]
    fn test_parse_alignment() {
        let xml = r#"<e horizontal="CENTER"/>"#;
        let mut reader = Reader::from_str(xml);
        let mut buf = Vec::new();
        if let Ok(Event::Empty(ref e)) = reader.read_event_into(&mut buf) {
            for attr in e.attributes().flatten() {
                if attr.key.as_ref() == b"horizontal" {
                    assert_eq!(parse_alignment(&attr), Alignment::Center);
                }
            }
        }
    }
}
