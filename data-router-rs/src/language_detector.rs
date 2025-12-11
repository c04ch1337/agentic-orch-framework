// data-router-rs/src/language_detector.rs
// NLP Language Detection using whatlang
// Identifies language for routing and tagging multi-lingual requests

use whatlang::{Info, Lang, detect};

/// Language detection result
#[derive(Debug, Clone)]
pub struct LanguageInfo {
    pub language: String,
    pub language_code: String,
    pub confidence: f64,
    pub is_reliable: bool,
}

impl Default for LanguageInfo {
    fn default() -> Self {
        Self {
            language: "unknown".to_string(),
            language_code: "und".to_string(),
            confidence: 0.0,
            is_reliable: false,
        }
    }
}

/// Detect the language of the given text
pub fn detect_language(text: &str) -> LanguageInfo {
    if text.trim().is_empty() || text.len() < 10 {
        return LanguageInfo::default();
    }

    match detect(text) {
        Some(info) => {
            let (lang_name, lang_code) = get_language_info(info.lang());
            LanguageInfo {
                language: lang_name,
                language_code: lang_code,
                confidence: info.confidence(),
                is_reliable: info.is_reliable(),
            }
        }
        None => LanguageInfo::default(),
    }
}

/// Check if text is in a specific language
pub fn is_language(text: &str, target_lang: &str) -> bool {
    let detected = detect_language(text);
    detected.language_code.eq_ignore_ascii_case(target_lang)
}

/// Get language name and ISO code
fn get_language_info(lang: Lang) -> (String, String) {
    match lang {
        Lang::Eng => ("English".to_string(), "en".to_string()),
        Lang::Spa => ("Spanish".to_string(), "es".to_string()),
        Lang::Fra => ("French".to_string(), "fr".to_string()),
        Lang::Deu => ("German".to_string(), "de".to_string()),
        Lang::Ita => ("Italian".to_string(), "it".to_string()),
        Lang::Por => ("Portuguese".to_string(), "pt".to_string()),
        Lang::Rus => ("Russian".to_string(), "ru".to_string()),
        Lang::Jpn => ("Japanese".to_string(), "ja".to_string()),
        Lang::Cmn => ("Chinese".to_string(), "zh".to_string()),
        Lang::Kor => ("Korean".to_string(), "ko".to_string()),
        Lang::Ara => ("Arabic".to_string(), "ar".to_string()),
        Lang::Hin => ("Hindi".to_string(), "hi".to_string()),
        Lang::Nld => ("Dutch".to_string(), "nl".to_string()),
        Lang::Pol => ("Polish".to_string(), "pl".to_string()),
        Lang::Tur => ("Turkish".to_string(), "tr".to_string()),
        Lang::Vie => ("Vietnamese".to_string(), "vi".to_string()),
        Lang::Tha => ("Thai".to_string(), "th".to_string()),
        Lang::Ukr => ("Ukrainian".to_string(), "uk".to_string()),
        Lang::Heb => ("Hebrew".to_string(), "he".to_string()),
        Lang::Swe => ("Swedish".to_string(), "sv".to_string()),
        _ => (format!("{:?}", lang), "und".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_english_detection() {
        let result = detect_language("This is a test message in English language.");
        assert_eq!(result.language_code, "en");
        assert!(result.is_reliable);
    }

    #[test]
    fn test_spanish_detection() {
        let result = detect_language("Este es un mensaje de prueba en español.");
        assert_eq!(result.language_code, "es");
    }

    #[test]
    fn test_short_text() {
        let result = detect_language("Hi");
        assert_eq!(result.language_code, "und");
    }
}
