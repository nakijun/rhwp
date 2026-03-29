//! 역공학용 HWP 샘플 자동 생성 테스트
//!
//! 기존 HWP 파일을 템플릿으로 로드하고 텍스트를 교체하여
//! 통제된 역공학 샘플을 생성한다.
//! 생성된 파일은 작업지시자가 한컴에서 열어 검증한다.

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::fs;

    /// 템플릿 HWP 로드 → 텍스트 교체 → 저장
    fn generate_sample(
        template_path: &str,
        output_path: &str,
        texts: &[&str],
    ) -> Result<(), String> {
        generate_sample_with_options(template_path, output_path, texts, None, None)
    }

    /// 폰트만 변경
    fn generate_sample_with_font(
        template_path: &str,
        output_path: &str,
        texts: &[&str],
        font_name: Option<&str>,
    ) -> Result<(), String> {
        generate_sample_with_options(template_path, output_path, texts, font_name, None)
    }

    /// empty.hwp 템플릿에 DocumentCore API로 텍스트 삽입하여 샘플 생성
    fn generate_sample_with_options(
        _template_path: &str,
        output_path: &str,
        texts: &[&str],
        font_name: Option<&str>,
        alignment: Option<crate::model::style::Alignment>,
    ) -> Result<(), String> {
        // 빈 문서 템플릿 로드 (컨트롤 마커 구조가 올바른 상태)
        let empty_path = Path::new("template/empty.hwp");
        if !empty_path.exists() {
            return Err("template/empty.hwp 없음".to_string());
        }
        let data = fs::read(empty_path).map_err(|e| e.to_string())?;
        let mut core = crate::document_core::DocumentCore::from_bytes(&data)
            .map_err(|e| format!("{:?}", e))?;

        // 폰트 변경
        if let Some(fname) = font_name {
            for lang_fonts in &mut core.document.doc_info.font_faces {
                if lang_fonts.is_empty() {
                    lang_fonts.push(crate::model::style::Font {
                        raw_data: None,
                        name: fname.to_string(),
                        alt_type: 0,
                        alt_name: None,
                        default_name: None,
                    });
                } else {
                    lang_fonts[0].name = fname.to_string();
                    lang_fonts[0].raw_data = None;
                }
            }
            core.document.doc_info.raw_stream = None;
        }

        // 정렬 변경
        if let Some(align) = alignment {
            let para = &core.document.sections[0].paragraphs[0];
            let ps_id = para.para_shape_id as usize;
            if ps_id < core.document.doc_info.para_shapes.len() {
                core.document.doc_info.para_shapes[ps_id].alignment = align;
                core.document.doc_info.para_shapes[ps_id].raw_data = None;
                core.document.doc_info.raw_stream = None;
            }
        }

        // 첫 문단에 텍스트 삽입 (DocumentCore API 사용)
        if let Some(first_text) = texts.first() {
            let _ = core.insert_text_native(0, 0, 0, first_text);
        }

        // 추가 문단: Enter로 문단 분할 후 텍스트 삽입
        for (i, text) in texts.iter().enumerate().skip(1) {
            // 이전 문단 끝에서 Enter
            let prev_para = &core.document.sections[0].paragraphs[i - 1];
            let end_pos = prev_para.text.chars().count();
            let _ = core.split_paragraph_native(0, i - 1, end_pos);
            // 새 문단에 텍스트 삽입
            let _ = core.insert_text_native(0, i, 0, text);
        }

        // raw_stream 무효화
        core.document.sections[0].raw_stream = None;

        // 직렬화
        let bytes = crate::serializer::serialize_document(&core.document)
            .map_err(|e| format!("{:?}", e))?;
        fs::write(output_path, bytes).map_err(|e| e.to_string())?;

        eprintln!("생성: {} ({}문단)", output_path, texts.len());
        Ok(())
    }

    /// 한글만 반복하여 지정 길이의 텍스트 생성
    fn hangul_repeat(pattern: &str, target_chars: usize) -> String {
        let chars: Vec<char> = pattern.chars().collect();
        let mut result = String::new();
        for i in 0..target_chars {
            result.push(chars[i % chars.len()]);
        }
        result
    }

    /// 한글+공백 패턴 생성 ("가 나 다 라 ...")
    fn hangul_with_spaces(pattern: &str, target_chars: usize) -> String {
        let chars: Vec<char> = pattern.chars().collect();
        let mut result = String::new();
        let mut count = 0;
        let mut ci = 0;
        while count < target_chars {
            result.push(chars[ci % chars.len()]);
            count += 1;
            if count < target_chars {
                result.push(' ');
                count += 1;
            }
            ci += 1;
        }
        result
    }

    // ─── 1차 샘플: 기본 폭 측정 ───

    #[test]
    fn test_gen_re01_hangul_only() {
        // 한글만 반복 (공백 없음), 2~3줄 분량
        // A4 바탕체 10pt: 한 줄 약 43자 → 100자면 ~2.3줄
        let text = hangul_repeat("가나다라마바사아자차카타파하", 100);
        let result = generate_sample(
            "samples/lseg-01-basic.hwp",
            "samples/re-01-hangul-only.hwp",
            &[&text],
        );
        if let Err(e) = result {
            eprintln!("re-01 생성 실패: {}", e);
        }
    }

    #[test]
    fn test_gen_re02_space_count() {
        // 한글+공백 ("가 나 다 라 ..."), 2~3줄
        let text = hangul_with_spaces("가나다라마바사아자차카타파하", 100);
        let result = generate_sample(
            "samples/lseg-01-basic.hwp",
            "samples/re-02-space-count.hwp",
            &[&text],
        );
        if let Err(e) = result {
            eprintln!("re-02 생성 실패: {}", e);
        }
    }

    #[test]
    fn test_gen_re03_latin_only() {
        // 영문만 반복, 2~3줄
        let text = "abcdefghijklmnopqrstuvwxyz".repeat(8); // 208자
        let result = generate_sample(
            "samples/lseg-01-basic.hwp",
            "samples/re-03-latin-only.hwp",
            &[&text],
        );
        if let Err(e) = result {
            eprintln!("re-03 생성 실패: {}", e);
        }
    }

    #[test]
    fn test_gen_re04_digit_only() {
        // 숫자만 반복, 2~3줄
        let text = "1234567890".repeat(20); // 200자
        let result = generate_sample(
            "samples/lseg-01-basic.hwp",
            "samples/re-04-digit-only.hwp",
            &[&text],
        );
        if let Err(e) = result {
            eprintln!("re-04 생성 실패: {}", e);
        }
    }

    #[test]
    fn test_gen_re05_mixed_koen() {
        // 한영 혼합 반복, 2~3줄
        let base = "한글English한글English";
        let text = base.repeat(8);
        let result = generate_sample(
            "samples/lseg-01-basic.hwp",
            "samples/re-05-mixed-koen.hwp",
            &[&text],
        );
        if let Err(e) = result {
            eprintln!("re-05 생성 실패: {}", e);
        }
    }

    #[test]
    fn test_gen_re06_punctuation() {
        // 구두점 포함 한글, 2~3줄
        let base = "가,나.다!라?마(바)사[아]자{차}";
        let text = base.repeat(5);
        let result = generate_sample(
            "samples/lseg-01-basic.hwp",
            "samples/re-06-punctuation.hwp",
            &[&text],
        );
        if let Err(e) = result {
            eprintln!("re-06 생성 실패: {}", e);
        }
    }

    // ─── 폰트별 샘플 ───

    #[test]
    fn test_gen_re_font_variations() {
        let fonts = [
            ("batang", "바탕"),
            ("batangche", "바탕체"),
            ("gulim", "굴림"),
            ("gulimche", "굴림체"),
            ("dotum", "돋움"),
            ("dotumche", "돋움체"),
            ("malgun", "맑은 고딕"),
        ];

        // 동일한 테스트 텍스트 (한글+영문+숫자+구두점 혼합)
        let text = "가나다라 English 12345 가,나.다! 마바사아 test 67890 자차카타파하";
        let long_text = format!("{} {}", text, text); // 2줄 이상

        for (suffix, font_name) in &fonts {
            let output = format!("samples/re-font-{}.hwp", suffix);
            let result = generate_sample_with_font(
                "samples/lseg-01-basic.hwp",
                &output,
                &[&long_text],
                Some(font_name),
            );
            match result {
                Ok(()) => eprintln!("생성: {} (폰트: {})", output, font_name),
                Err(e) => eprintln!("실패: {} — {}", output, e),
            }
        }
    }

    // ─── 정렬별 샘플 ───

    #[test]
    fn test_gen_re_alignment_variations() {
        use crate::model::style::Alignment;

        let aligns = [
            ("justify", Alignment::Justify),
            ("left", Alignment::Left),
            ("center", Alignment::Center),
            ("right", Alignment::Right),
        ];

        let text = hangul_repeat("가나다라마바사아자차카타파하", 100);

        for (suffix, alignment) in &aligns {
            let output = format!("samples/re-align-{}.hwp", suffix);
            let result = generate_sample_with_options(
                "samples/lseg-01-basic.hwp",
                &output,
                &[&text],
                None,
                Some(*alignment),
            );
            match result {
                Ok(()) => eprintln!("생성: {} (정렬: {:?})", output, alignment),
                Err(e) => eprintln!("실패: {} — {}", output, e),
            }
        }
    }

    // ─── 일괄 생성 ───

    #[test]
    fn test_gen_all_re_samples() {
        test_gen_re01_hangul_only();
        test_gen_re02_space_count();
        test_gen_re03_latin_only();
        test_gen_re04_digit_only();
        test_gen_re05_mixed_koen();
        test_gen_re06_punctuation();
        eprintln!("\n=== 1차 샘플 생성 완료 ===");
        eprintln!("검증 필요: samples/re-01 ~ re-06.hwp를 한컴에서 열어 확인");
    }
}
