mod expression;
mod matching;
mod tokens;

use expression::expand_masks;
use matching::matches_tokens;
use tokens::{Token, tokenize};

const MAX_PATTERN_LENGTH: usize = 4096;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlobMatcher {
    masks: Vec<Mask>,
    has_inclusions: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Mask {
    excluded: bool,
    directory_only: bool,
    matches_path: bool,
    tokens: Vec<Token>,
}

impl GlobMatcher {
    pub fn parse(expression: &str) -> Result<Self, String> {
        if expression.is_empty() {
            return Err("finder pattern cannot be empty".into());
        }

        if expression.len() > MAX_PATTERN_LENGTH {
            return Err(format!(
                "finder pattern exceeds the {MAX_PATTERN_LENGTH}-byte limit"
            ));
        }

        let expanded = expand_masks(expression)?;
        let mut masks = Vec::with_capacity(expanded.len());
        for raw in expanded {
            masks.push(Mask::parse(&raw)?);
        }

        let has_inclusions = masks.iter().any(|mask| !mask.excluded);

        Ok(Self {
            masks,
            has_inclusions,
        })
    }

    pub fn matches(&self, relative_path: &str, basename: &str, is_directory: bool) -> bool {
        let relative_path = relative_path.replace('\\', "/");
        let mut included = !self.has_inclusions;
        for mask in &self.masks {
            if !mask.matches(&relative_path, basename, is_directory) {
                continue;
            }

            if mask.excluded {
                return false;
            }

            included = true;
        }

        included
    }
}

impl Mask {
    fn parse(raw: &str) -> Result<Self, String> {
        let (excluded, raw) = raw
            .strip_prefix('!')
            .map_or((false, raw), |value| (true, value));
        if raw.is_empty() {
            return Err("glob exclusion is missing a pattern after '!'".into());
        }

        let normalized = raw.replace('\\', "/");
        let directory_only = normalized.ends_with('/');
        let pattern = normalized.trim_end_matches('/');
        if pattern.is_empty() {
            return Err("directory-only glob is missing a name".into());
        }

        Ok(Self {
            excluded,
            directory_only,
            matches_path: pattern.contains('/'),
            tokens: tokenize(pattern)?,
        })
    }

    fn matches(&self, relative_path: &str, basename: &str, is_directory: bool) -> bool {
        if self.directory_only && !is_directory {
            return false;
        }

        let candidate = if self.matches_path {
            relative_path
        } else {
            basename
        };

        matches_tokens(&self.tokens, candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches(pattern: &str, path: &str, directory: bool) -> bool {
        let basename = path.rsplit('/').next().unwrap_or(path);
        GlobMatcher::parse(pattern)
            .unwrap()
            .matches(path, basename, directory)
    }

    #[test]
    fn basename_and_path_globs_support_star_question_and_globstar() {
        assert!(matches("*.cpp", "Source/Foo.cpp", false));
        assert!(matches("Source/*.cpp", "Source/Foo.cpp", false));
        assert!(!matches("Source/*.cpp", "Source/Nested/Foo.cpp", false));

        assert!(matches(
            "Source/**/Command*.cpp",
            "Source/Command.cpp",
            false
        ));
        assert!(matches(
            "Source/**/Command*.cpp",
            "Source/Nested/CommandBar.cpp",
            false
        ));
        assert!(matches("**/*.rs", "lib.rs", false));

        assert!(matches("File?.[ch]", "File猫.c", false));
        assert!(!matches("File?.[ch]", "File猫狗.c", false));
    }

    #[test]
    fn classes_braces_masks_and_exclusions_are_composable() {
        let matcher = GlobMatcher::parse("Test[abc].{cpp,{h,hpp}};!Testb.*").unwrap();

        assert!(matcher.matches("Testa.cpp", "Testa.cpp", false));
        assert!(matcher.matches("Testc.hpp", "Testc.hpp", false));
        assert!(!matcher.matches("Testb.h", "Testb.h", false));

        assert!(!matches("Test[!abc].cpp", "Testa.cpp", false));
        assert!(matches("Test[^abc].cpp", "Testz.cpp", false));
        assert!(matches("[a-z].txt", "m.txt", false));
    }

    #[test]
    fn exclusions_only_and_directory_suffixes_use_normalized_separators() {
        let matcher = GlobMatcher::parse("!Build/**;!*.tmp").unwrap();

        assert!(matcher.matches("Source/Foo.cpp", "Foo.cpp", false));
        assert!(!matcher.matches("Build/Foo.cpp", "Foo.cpp", false));
        assert!(!matcher.matches("Source/Foo.tmp", "Foo.tmp", false));

        assert!(matches("Source/", "Nested/Source", true));
        assert!(!matches("Source/", "Nested/Source", false));
        assert!(matches("*\\Tests\\", "Project/Tests", true));
    }

    #[test]
    fn invalid_and_pathological_patterns_are_bounded() {
        assert!(GlobMatcher::parse("[abc").is_err());
        assert!(GlobMatcher::parse("{cpp,h").is_err());
        assert!(GlobMatcher::parse(&"x".repeat(MAX_PATTERN_LENGTH + 1)).is_err());

        let explosive = (0..11).fold(String::new(), |mut value, _| {
            value.push_str("{a,b}");
            value
        });
        assert!(GlobMatcher::parse(&explosive).is_err());

        let matcher = GlobMatcher::parse("**a**a**a**a**a**b").unwrap();
        assert!(!matcher.matches(&"a".repeat(500), "", false));
    }
}
