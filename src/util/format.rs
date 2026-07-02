use regex::Regex;

/**
 * Convert an arbitrary string to kebab-case, generally used for URL transforms.
 */
pub fn to_kebab_case(input: &str) -> String {
    let non_kebab_characters_regex = Regex::new(r"[()\[\]!?'&#.,/\\~+]").unwrap();
    non_kebab_characters_regex.replace_all(input, "")
        .split_whitespace()
        .flat_map(|s| s.split('_'))
        .flat_map(|s| s.split('-'))
        .filter(|s| !s.is_empty())
        .map(|word| word.to_lowercase())
        .collect::<Vec<_>>()
        .join("-")
}

/**
 * Cut off the string at max character length
 */
pub fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}
