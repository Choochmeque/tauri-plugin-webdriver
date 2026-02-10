/// Locator strategies for finding elements
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocatorStrategy {
    CssSelector,
    LinkText,
    PartialLinkText,
    TagName,
    XPath,
}

impl LocatorStrategy {
    /// Parse locator strategy from WebDriver string
    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            "css selector" => Some(Self::CssSelector),
            "link text" => Some(Self::LinkText),
            "partial link text" => Some(Self::PartialLinkText),
            "tag name" => Some(Self::TagName),
            "xpath" => Some(Self::XPath),
            _ => None,
        }
    }

    /// Generate JavaScript expression to find element (just the selector, no wrapper)
    pub fn to_selector_js(&self, value: &str) -> String {
        let escaped = value.replace('\\', "\\\\").replace('\'', "\\'");

        match self {
            LocatorStrategy::CssSelector => {
                format!("document.querySelector('{}')", escaped)
            }
            LocatorStrategy::TagName => {
                format!("document.getElementsByTagName('{}')[0] || null", escaped)
            }
            LocatorStrategy::XPath => {
                format!(
                    r#"(function() {{
                        var result = document.evaluate('{}', document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null);
                        return result.singleNodeValue;
                    }})()"#,
                    escaped
                )
            }
            LocatorStrategy::LinkText => {
                format!(
                    r#"Array.from(document.querySelectorAll('a')).find(a => a.textContent.trim() === '{}') || null"#,
                    escaped
                )
            }
            LocatorStrategy::PartialLinkText => {
                format!(
                    r#"Array.from(document.querySelectorAll('a')).find(a => a.textContent.includes('{}')) || null"#,
                    escaped
                )
            }
        }
    }

    /// Generate JavaScript expression to find multiple elements
    pub fn to_selector_js_multiple(&self, value: &str) -> String {
        let escaped = value.replace('\\', "\\\\").replace('\'', "\\'");

        match self {
            LocatorStrategy::CssSelector => {
                format!("Array.from(document.querySelectorAll('{}'))", escaped)
            }
            LocatorStrategy::TagName => {
                format!("Array.from(document.getElementsByTagName('{}'))", escaped)
            }
            LocatorStrategy::XPath => {
                format!(
                    r#"(function() {{
                        var result = [];
                        var iter = document.evaluate('{}', document, null, XPathResult.ORDERED_NODE_ITERATOR_TYPE, null);
                        var node;
                        while ((node = iter.iterateNext())) {{
                            result.push(node);
                        }}
                        return result;
                    }})()"#,
                    escaped
                )
            }
            LocatorStrategy::LinkText => {
                format!(
                    r#"Array.from(document.querySelectorAll('a')).filter(a => a.textContent.trim() === '{}')"#,
                    escaped
                )
            }
            LocatorStrategy::PartialLinkText => {
                format!(
                    r#"Array.from(document.querySelectorAll('a')).filter(a => a.textContent.includes('{}'))"#,
                    escaped
                )
            }
        }
    }

    /// Generate JavaScript expression to find a single element from a parent element
    /// Returns an expression that evaluates to a single element (or null)
    /// Assumes `parent` variable is defined
    pub fn to_selector_js_single_from_element(&self, value: &str) -> String {
        let escaped = value.replace('\\', "\\\\").replace('\'', "\\'");

        match self {
            LocatorStrategy::CssSelector => {
                format!("parent.querySelector('{}')", escaped)
            }
            LocatorStrategy::TagName => {
                format!("parent.getElementsByTagName('{}')[0] || null", escaped)
            }
            LocatorStrategy::XPath => {
                format!(
                    r#"(function() {{
                        var result = document.evaluate('{}', parent, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null);
                        return result.singleNodeValue;
                    }})()"#,
                    escaped
                )
            }
            LocatorStrategy::LinkText => {
                format!(
                    r#"Array.from(parent.querySelectorAll('a')).find(a => a.textContent.trim() === '{}') || null"#,
                    escaped
                )
            }
            LocatorStrategy::PartialLinkText => {
                format!(
                    r#"Array.from(parent.querySelectorAll('a')).find(a => a.textContent.includes('{}')) || null"#,
                    escaped
                )
            }
        }
    }

    /// Generate JavaScript expression to find multiple elements from a parent element
    /// Returns an expression that evaluates to an array-like collection
    /// Assumes `parent` variable is defined
    pub fn to_selector_js_from_element(&self, value: &str) -> String {
        let escaped = value.replace('\\', "\\\\").replace('\'', "\\'");

        match self {
            LocatorStrategy::CssSelector => {
                format!("Array.from(parent.querySelectorAll('{}'))", escaped)
            }
            LocatorStrategy::TagName => {
                format!("Array.from(parent.getElementsByTagName('{}'))", escaped)
            }
            LocatorStrategy::XPath => {
                format!(
                    r#"(function() {{
                        var result = [];
                        var iter = document.evaluate('{}', parent, null, XPathResult.ORDERED_NODE_ITERATOR_TYPE, null);
                        var node;
                        while ((node = iter.iterateNext())) {{
                            result.push(node);
                        }}
                        return result;
                    }})()"#,
                    escaped
                )
            }
            LocatorStrategy::LinkText => {
                format!(
                    r#"Array.from(parent.querySelectorAll('a')).filter(a => a.textContent.trim() === '{}')"#,
                    escaped
                )
            }
            LocatorStrategy::PartialLinkText => {
                format!(
                    r#"Array.from(parent.querySelectorAll('a')).filter(a => a.textContent.includes('{}'))"#,
                    escaped
                )
            }
        }
    }

    /// Generate JavaScript expression to find a single element from a shadow root
    /// Returns an expression that evaluates to a single element (or null)
    /// Assumes `shadow` variable is defined
    pub fn to_selector_js_single_from_shadow(&self, value: &str) -> String {
        let escaped = value.replace('\\', "\\\\").replace('\'', "\\'");

        match self {
            LocatorStrategy::CssSelector => {
                format!("shadow.querySelector('{}')", escaped)
            }
            LocatorStrategy::TagName => {
                format!("shadow.querySelector('{}')", escaped)
            }
            LocatorStrategy::XPath => {
                // XPath from shadow root context
                format!(
                    r#"(function() {{
                        var result = document.evaluate('{}', shadow, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null);
                        return result.singleNodeValue;
                    }})()"#,
                    escaped
                )
            }
            LocatorStrategy::LinkText => {
                format!(
                    r#"Array.from(shadow.querySelectorAll('a')).find(a => a.textContent.trim() === '{}') || null"#,
                    escaped
                )
            }
            LocatorStrategy::PartialLinkText => {
                format!(
                    r#"Array.from(shadow.querySelectorAll('a')).find(a => a.textContent.includes('{}')) || null"#,
                    escaped
                )
            }
        }
    }

    /// Generate JavaScript expression to find multiple elements from a shadow root
    /// Returns an expression that evaluates to an array-like collection
    /// Assumes `shadow` variable is defined
    pub fn to_selector_js_from_shadow(&self, value: &str) -> String {
        let escaped = value.replace('\\', "\\\\").replace('\'', "\\'");

        match self {
            LocatorStrategy::CssSelector => {
                format!("Array.from(shadow.querySelectorAll('{}'))", escaped)
            }
            LocatorStrategy::TagName => {
                format!("Array.from(shadow.querySelectorAll('{}'))", escaped)
            }
            LocatorStrategy::XPath => {
                format!(
                    r#"(function() {{
                        var result = [];
                        var iter = document.evaluate('{}', shadow, null, XPathResult.ORDERED_NODE_ITERATOR_TYPE, null);
                        var node;
                        while ((node = iter.iterateNext())) {{
                            result.push(node);
                        }}
                        return result;
                    }})()"#,
                    escaped
                )
            }
            LocatorStrategy::LinkText => {
                format!(
                    r#"Array.from(shadow.querySelectorAll('a')).filter(a => a.textContent.trim() === '{}')"#,
                    escaped
                )
            }
            LocatorStrategy::PartialLinkText => {
                format!(
                    r#"Array.from(shadow.querySelectorAll('a')).filter(a => a.textContent.includes('{}'))"#,
                    escaped
                )
            }
        }
    }

    /// Generate JavaScript code to find element(s) and store in global variable
    pub fn to_find_js(&self, value: &str, multiple: bool, js_var: &str) -> String {
        let escaped = value.replace('\\', "\\\\").replace('\'', "\\'");

        let find_expr = match self {
            LocatorStrategy::CssSelector => {
                if multiple {
                    format!("Array.from(document.querySelectorAll('{}'))", escaped)
                } else {
                    format!("document.querySelector('{}')", escaped)
                }
            }
            LocatorStrategy::TagName => {
                if multiple {
                    format!("Array.from(document.getElementsByTagName('{}'))", escaped)
                } else {
                    format!("document.getElementsByTagName('{}')[0] || null", escaped)
                }
            }
            LocatorStrategy::XPath => {
                if multiple {
                    format!(
                        r#"(function() {{
                            var result = [];
                            var iter = document.evaluate('{}', document, null, XPathResult.ORDERED_NODE_ITERATOR_TYPE, null);
                            var node;
                            while ((node = iter.iterateNext())) {{
                                result.push(node);
                            }}
                            return result;
                        }})()"#,
                        escaped
                    )
                } else {
                    format!(
                        r#"(function() {{
                            var result = document.evaluate('{}', document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null);
                            return result.singleNodeValue;
                        }})()"#,
                        escaped
                    )
                }
            }
            LocatorStrategy::LinkText => {
                if multiple {
                    format!(
                        r#"Array.from(document.querySelectorAll('a')).filter(a => a.textContent.trim() === '{}')"#,
                        escaped
                    )
                } else {
                    format!(
                        r#"Array.from(document.querySelectorAll('a')).find(a => a.textContent.trim() === '{}') || null"#,
                        escaped
                    )
                }
            }
            LocatorStrategy::PartialLinkText => {
                if multiple {
                    format!(
                        r#"Array.from(document.querySelectorAll('a')).filter(a => a.textContent.includes('{}'))"#,
                        escaped
                    )
                } else {
                    format!(
                        r#"Array.from(document.querySelectorAll('a')).find(a => a.textContent.includes('{}')) || null"#,
                        escaped
                    )
                }
            }
        };

        // Store the found element(s) in a global variable
        format!(
            r#"(function() {{
                var el = {};
                if (el) {{
                    window.{} = el;
                    return true;
                }}
                return false;
            }})()"#,
            find_expr, js_var
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_strategy() {
        assert_eq!(
            LocatorStrategy::from_string("css selector"),
            Some(LocatorStrategy::CssSelector)
        );
        assert_eq!(
            LocatorStrategy::from_string("xpath"),
            Some(LocatorStrategy::XPath)
        );
        assert_eq!(LocatorStrategy::from_string("unknown"), None);
    }

    #[test]
    fn test_css_selector_js() {
        let strategy = LocatorStrategy::CssSelector;
        let js = strategy.to_find_js("#my-button", false, "__wd_el_0");

        assert!(js.contains("querySelector"));
        assert!(js.contains("#my-button"));
        assert!(js.contains("__wd_el_0"));
    }

    #[test]
    fn test_xpath_js() {
        let strategy = LocatorStrategy::XPath;
        let js = strategy.to_find_js("//div[@id='test']", false, "__wd_el_1");

        assert!(js.contains("document.evaluate"));
        assert!(js.contains("//div[@id='test']"));
    }

    #[test]
    fn test_escaping() {
        let strategy = LocatorStrategy::CssSelector;
        let js = strategy.to_find_js("div[data-value='test']", false, "__wd_el_0");

        assert!(js.contains("div[data-value=\\'test\\']"));
    }
}
