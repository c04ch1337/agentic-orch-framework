//! HTML sanitization utilities
//!
//! This module provides sanitizers for HTML content, helping prevent
//! XSS (Cross-Site Scripting) and other HTML-based attacks.

use super::SanitizeResult;
use lazy_static::lazy_static;
use std::collections::HashSet;

lazy_static! {
    /// Set of allowed HTML tags for basic HTML sanitization
    static ref SAFE_TAGS: HashSet<&'static str> = {
        let mut tags = HashSet::new();
        tags.insert("p");
        tags.insert("br");
        tags.insert("b");
        tags.insert("i");
        tags.insert("em");
        tags.insert("strong");
        tags.insert("a");
        tags.insert("ul");
        tags.insert("ol");
        tags.insert("li");
        tags.insert("span");
        tags.insert("div");
        tags.insert("blockquote");
        tags.insert("code");
        tags.insert("pre");
        tags.insert("h1");
        tags.insert("h2");
        tags.insert("h3");
        tags.insert("h4");
        tags.insert("h5");
        tags.insert("h6");
        tags.insert("img");
        tags
    };

    /// Set of allowed HTML attributes for basic HTML sanitization
    static ref SAFE_ATTRS: HashSet<&'static str> = {
        let mut attrs = HashSet::new();
        attrs.insert("href");
        attrs.insert("src");
        attrs.insert("alt");
        attrs.insert("title");
        attrs.insert("class");
        attrs.insert("id");
        attrs.insert("style");
        attrs
    };
}

/// Encode HTML special characters to prevent XSS
pub fn encode_html_entities(input: &str) -> SanitizeResult<String> {
    let replacements = [
        ('&', "&amp;"),
        ('<', "&lt;"),
        ('>', "&gt;"),
        ('"', "&quot;"),
        ('\'', "&#39;"),
    ];

    let mut result = input.to_string();
    let mut was_modified = false;

    for (from, to) in &replacements {
        let original_len = result.len();
        result = result.replace(*from, to);
        if result.len() != original_len {
            was_modified = true;
        }
    }

    if was_modified {
        SanitizeResult::modified(result, Some("Encoded HTML entities".to_string()))
    } else {
        SanitizeResult::unmodified(input.to_string())
    }
}

/// Decode HTML entities to their original characters
pub fn decode_html_entities(input: &str) -> SanitizeResult<String> {
    let replacements = [
        ("&amp;", "&"),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&#39;", "'"),
        ("&#x27;", "'"),
    ];

    let mut result = input.to_string();
    let mut was_modified = false;

    for (from, to) in &replacements {
        let original_len = result.len();
        result = result.replace(from, to);
        if result.len() != original_len {
            was_modified = true;
        }
    }

    if was_modified {
        SanitizeResult::modified(result, Some("Decoded HTML entities".to_string()))
    } else {
        SanitizeResult::unmodified(input.to_string())
    }
}

/// Remove all HTML tags from the input string
pub fn strip_html_tags(input: &str) -> SanitizeResult<String> {
    lazy_static! {
        static ref TAG_REGEX: regex::Regex = regex::Regex::new(r"<[^>]*>").unwrap();
    }

    let result = TAG_REGEX.replace_all(input, "").to_string();

    if result == input {
        SanitizeResult::unmodified(input.to_string())
    } else {
        SanitizeResult::modified(result, Some("Removed HTML tags".to_string()))
    }
}

/// Simple HTML sanitization that only allows safe tags and attributes
/// Note: For production use, consider using a dedicated HTML sanitization library
/// This is a simple implementation for basic cases
pub fn sanitize_html(input: &str) -> SanitizeResult<String> {
    lazy_static! {
        // Match HTML tags with attributes
        static ref TAG_ATTR_REGEX: regex::Regex = regex::Regex::new(
            r"<(/?)([a-zA-Z][a-zA-Z0-9]*)([^>]*)(/?)>"
        ).unwrap();

        // Match attributes within a tag
        static ref ATTR_REGEX: regex::Regex = regex::Regex::new(
            r#"([a-zA-Z][a-zA-Z0-9\-_]*)(?:\s*=\s*(?:(?:"([^"]*)")|(?:'([^']*)')|([^\s>]+)))?"#
        ).unwrap();

        // Match potentially dangerous attributes
        static ref DANGEROUS_ATTRS_REGEX: regex::Regex = regex::Regex::new(
            r#"(?i)(on\w+|javascript:|data:)"#
        ).unwrap();
    }

    let mut result = input.to_string();
    let mut was_modified = false;

    // Process HTML tags
    let sanitized = TAG_ATTR_REGEX
        .replace_all(&result, |caps: &regex::Captures| {
            let closing = &caps[1];
            let tag_name = &caps[2].to_lowercase();
            let attrs = &caps[3];
            let self_closing = &caps[4];

            // Check if the tag is allowed
            if !SAFE_TAGS.contains(&tag_name.as_str()) {
                was_modified = true;
                return "".to_string();
            }

            // Process attributes if there are any
            let sanitized_attrs = if !attrs.is_empty() {
                let mut safe_attrs = String::new();

                for attr_caps in ATTR_REGEX.captures_iter(attrs) {
                    let attr_name = attr_caps[1].to_lowercase();

                    // Skip unsafe attributes or attributes with unsafe values
                    if !SAFE_ATTRS.contains(&attr_name.as_str())
                        || DANGEROUS_ATTRS_REGEX.is_match(&attr_caps[0])
                    {
                        was_modified = true;
                        continue;
                    }

                    // Get the attribute value
                    let attr_value = if attr_caps.get(2).is_some() {
                        &attr_caps[2] // Double-quoted value
                    } else if attr_caps.get(3).is_some() {
                        &attr_caps[3] // Single-quoted value
                    } else if attr_caps.get(4).is_some() {
                        &attr_caps[4] // Unquoted value
                    } else {
                        "" // No value
                    };

                    // For href and src attributes, ensure they don't contain javascript:
                    if (attr_name == "href" || attr_name == "src")
                        && attr_value.to_lowercase().contains("javascript:")
                    {
                        was_modified = true;
                        continue;
                    }

                    safe_attrs.push_str(" ");
                    safe_attrs.push_str(&attr_name);

                    if !attr_value.is_empty() {
                        safe_attrs.push_str("=\"");
                        safe_attrs.push_str(attr_value);
                        safe_attrs.push_str("\"");
                    }
                }

                safe_attrs
            } else {
                attrs.to_string()
            };

            format!(
                "<{}{}{}{}>",
                closing, tag_name, sanitized_attrs, self_closing
            )
        })
        .to_string();

    if sanitized != result {
        was_modified = true;
    }

    result = sanitized;

    if was_modified {
        SanitizeResult::modified(result, Some("Sanitized HTML content".to_string()))
    } else {
        SanitizeResult::unmodified(input.to_string())
    }
}

/// Sanitize HTML for plaintext contexts (removes all HTML)
pub fn sanitize_for_plaintext(input: &str) -> SanitizeResult<String> {
    let result = strip_html_tags(input);

    if !result.was_modified {
        return SanitizeResult::unmodified(input.to_string());
    }

    let result = decode_html_entities(&result.sanitized);

    SanitizeResult::modified(
        result.sanitized,
        Some("Converted HTML to plaintext".to_string()),
    )
}

/// Completely strip all HTML-related content for maximum security
pub fn strict_html_sanitize(input: &str) -> SanitizeResult<String> {
    let stripped = strip_html_tags(input);
    let decoded = decode_html_entities(&stripped.sanitized);

    // Remove any potential script content
    lazy_static! {
        static ref SCRIPT_REGEX: regex::Regex =
            regex::Regex::new(r"(?i)javascript:|data:|vbscript:|expression\(|@import").unwrap();
    }

    let result = SCRIPT_REGEX
        .replace_all(&decoded.sanitized, "[removed]")
        .to_string();
    let was_modified = stripped.was_modified || decoded.was_modified || result != decoded.sanitized;

    if was_modified {
        SanitizeResult::modified(result, Some("Applied strict HTML sanitization".to_string()))
    } else {
        SanitizeResult::unmodified(input.to_string())
    }
}

/// Escape HTML for use in JavaScript strings
/// This prevents XSS when HTML is dynamically inserted
pub fn escape_html_for_javascript(input: &str) -> SanitizeResult<String> {
    let replacements = [
        ('\\', "\\\\"), // Must be first to avoid double-escaping
        ('\'', "\\'"),
        ('"', "\\\""),
        ('\n', "\\n"),
        ('\r', "\\r"),
        ('\t', "\\t"),
        ('<', "\\x3C"),
        ('>', "\\x3E"),
        ('&', "\\x26"),
    ];

    let mut result = input.to_string();
    let mut was_modified = false;

    for (from, to) in &replacements {
        let original_len = result.len();
        result = result.replace(*from, to);
        if result.len() != original_len {
            was_modified = true;
        }
    }

    if was_modified {
        SanitizeResult::modified(result, Some("Escaped HTML for JavaScript".to_string()))
    } else {
        SanitizeResult::unmodified(input.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_html_entities() {
        let input = "Test <script>alert('XSS')</script>";
        let result = encode_html_entities(input);

        assert!(result.was_modified);
        assert_eq!(
            result.sanitized,
            "Test &lt;script&gt;alert(&#39;XSS&#39;)&lt;/script&gt;"
        );

        // Test with input that doesn't need encoding
        let clean = "Plain text without special chars";
        let result = encode_html_entities(clean);

        assert!(!result.was_modified);
        assert_eq!(result.sanitized, clean);
    }

    #[test]
    fn test_decode_html_entities() {
        let input = "Test &lt;script&gt;alert(&quot;XSS&quot;)&lt;/script&gt;";
        let result = decode_html_entities(input);

        assert!(result.was_modified);
        assert_eq!(result.sanitized, "Test <script>alert(\"XSS\")</script>");

        // Test with input that doesn't need decoding
        let clean = "Plain text without entities";
        let result = decode_html_entities(clean);

        assert!(!result.was_modified);
        assert_eq!(result.sanitized, clean);
    }

    #[test]
    fn test_strip_html_tags() {
        let input = "<p>Test <strong>bold</strong> text</p>";
        let result = strip_html_tags(input);

        assert!(result.was_modified);
        assert_eq!(result.sanitized, "Test bold text");

        // Test with input that has no HTML
        let clean = "Plain text without HTML";
        let result = strip_html_tags(clean);

        assert!(!result.was_modified);
        assert_eq!(result.sanitized, clean);
    }

    #[test]
    fn test_sanitize_html() {
        // Test with safe HTML
        let safe =
            "<p>Test <strong>bold</strong> text with <a href=\"https://example.com\">link</a></p>";
        let result = sanitize_html(safe);
        assert!(!result.was_modified);
        assert_eq!(result.sanitized, safe);

        // Test with unsafe tags
        let unsafe_tags = "<p>Test <script>alert('XSS')</script> code</p>";
        let result = sanitize_html(unsafe_tags);
        assert!(result.was_modified);
        assert_eq!(result.sanitized, "<p>Test  code</p>");

        // Test with unsafe attributes
        let unsafe_attrs =
            "<p><a href=\"javascript:alert('XSS')\" onclick=\"evil()\">Bad Link</a></p>";
        let result = sanitize_html(unsafe_attrs);
        assert!(result.was_modified);
        assert_eq!(result.sanitized, "<p><a>Bad Link</a></p>");
    }

    #[test]
    fn test_sanitize_for_plaintext() {
        let input = "<p>Test <strong>bold</strong> with &quot;quotes&quot;</p>";
        let result = sanitize_for_plaintext(input);

        assert!(result.was_modified);
        assert_eq!(result.sanitized, "Test bold with \"quotes\"");
    }

    #[test]
    fn test_strict_html_sanitize() {
        let input = "<p>Test <script>alert('XSS')</script> with javascript:alert('Evil')</p>";
        let result = strict_html_sanitize(input);

        assert!(result.was_modified);
        assert_eq!(result.sanitized, "Test  with [removed]alert('Evil')");
    }

    #[test]
    fn test_escape_html_for_javascript() {
        let input = "Test <script>alert('XSS')</script>";
        let result = escape_html_for_javascript(input);

        assert!(result.was_modified);
        assert_eq!(
            result.sanitized,
            "Test \\x3Cscript\\x3Ealert(\\'XSS\\')\\x3C/script\\x3E"
        );
    }
}
