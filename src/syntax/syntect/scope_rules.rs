use syntect::highlighting::ScopeSelectors;
use syntect::parsing::ScopeStack;

use super::SyntaxCategory;

pub(super) fn load() -> Vec<(ScopeSelectors, SyntaxCategory)> {
    use SyntaxCategory::*;

    // Rule order breaks ties between equally specific selectors.
    let definitions = [
        ("comment, comment punctuation", Comment),
        ("keyword, storage", Keyword),
        ("string, punctuation.definition.string", String),
        ("constant.character.escape", Escape),
        ("constant.numeric", Number),
        ("constant.language", Boolean),
        (
            "entity.name.type, entity.name.class, support.type, support.class, storage.type",
            Type,
        ),
        (
            "entity.name.function, support.function, variable.function",
            Function,
        ),
        ("variable", Variable),
        ("variable.parameter", Parameter),
        (
            "support.type.property-name, variable.other.member, meta.mapping.key string, meta.structure.dictionary.key string",
            Property,
        ),
        ("constant, variable.other.constant", Constant),
        ("keyword.operator", Operator),
        ("punctuation", Punctuation),
        ("entity.name.tag", Tag),
        ("entity.other.attribute-name", Attribute),
        ("meta.preprocessor", Preprocessor),
        ("markup.heading, entity.name.section", Heading),
        (
            "markup.underline.link, string.other.link, meta.link.inline",
            Link,
        ),
        ("markup.raw", Code),
        ("markup.italic", Emphasis),
        ("markup.bold", Strong),
        ("markup.inserted", Added),
        ("markup.deleted", Removed),
        ("invalid", Invalid),
    ];

    definitions
        .into_iter()
        .map(|(selector, category)| {
            (
                selector.parse().expect("built-in syntax selector is valid"),
                category,
            )
        })
        .collect()
}

pub(super) fn category(
    rules: &[(ScopeSelectors, SyntaxCategory)],
    stack: &ScopeStack,
) -> SyntaxCategory {
    rules
        .iter()
        .filter_map(|(selector, category)| {
            selector
                .does_match(&stack.scopes)
                .map(|power| (power, *category))
        })
        .max_by_key(|(power, _)| *power)
        .map_or(SyntaxCategory::Text, |(_, category)| category)
}
