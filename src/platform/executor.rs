use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Write;

use crate::server::response::WebDriverErrorResponse;

/// Platform-agnostic trait for `WebView` operations.
/// Each platform (macOS, Windows, Linux) implements this trait.
#[async_trait]
pub trait PlatformExecutor: Send + Sync {
    // =========================================================================
    // Core JavaScript Execution
    // =========================================================================

    /// Execute JavaScript and return the result as JSON
    async fn evaluate_js(&self, script: &str) -> Result<Value, WebDriverErrorResponse>;

    // =========================================================================
    // Navigation
    // =========================================================================

    /// Navigate to a URL
    async fn navigate(&self, url: &str) -> Result<(), WebDriverErrorResponse> {
        let script = format!(
            r"window.location.href = '{}'; null;",
            url.replace('\\', "\\\\").replace('\'', "\\'")
        );
        self.evaluate_js(&script).await?;
        Ok(())
    }

    /// Get current URL
    async fn get_url(&self) -> Result<String, WebDriverErrorResponse> {
        let result = self.evaluate_js("window.location.href").await?;
        extract_string_value(&result)
    }

    /// Get page title
    async fn get_title(&self) -> Result<String, WebDriverErrorResponse> {
        let result = self.evaluate_js("document.title").await?;
        extract_string_value(&result)
    }

    /// Navigate back in history
    async fn go_back(&self) -> Result<(), WebDriverErrorResponse> {
        self.evaluate_js("window.history.back(); null;").await?;
        Ok(())
    }

    /// Navigate forward in history
    async fn go_forward(&self) -> Result<(), WebDriverErrorResponse> {
        self.evaluate_js("window.history.forward(); null;").await?;
        Ok(())
    }

    /// Refresh the current page
    async fn refresh(&self) -> Result<(), WebDriverErrorResponse> {
        self.evaluate_js("window.location.reload(); null;").await?;
        Ok(())
    }

    // =========================================================================
    // Document
    // =========================================================================

    /// Get page source HTML
    async fn get_source(&self) -> Result<String, WebDriverErrorResponse> {
        let result = self
            .evaluate_js("document.documentElement.outerHTML")
            .await?;
        extract_string_value(&result)
    }

    // =========================================================================
    // Element Operations
    // =========================================================================

    /// Find element and store reference in a JavaScript variable
    /// Returns true if element was found
    async fn find_element(
        &self,
        strategy_js: &str,
        js_var: &str,
    ) -> Result<bool, WebDriverErrorResponse> {
        let script = format!(
            r"(function() {{
                var el = {strategy_js};
                if (el) {{
                    window.{js_var} = el;
                    return true;
                }}
                return false;
            }})()"
        );
        let result = self.evaluate_js(&script).await?;
        extract_bool_value(&result)
    }

    /// Find multiple elements and store count
    /// Returns the number of elements found
    async fn find_elements(
        &self,
        strategy_js: &str,
        js_var_prefix: &str,
    ) -> Result<usize, WebDriverErrorResponse> {
        let script = format!(
            r"(function() {{
                var elements = {strategy_js};
                var count = elements.length;
                for (var i = 0; i < count; i++) {{
                    window['{js_var_prefix}' + i] = elements[i];
                }}
                return count;
            }})()"
        );
        let result = self.evaluate_js(&script).await?;
        extract_usize_value(&result)
    }

    /// Find element from a parent element and store reference
    /// Returns true if element was found
    async fn find_element_from_element(
        &self,
        parent_js_var: &str,
        strategy_js: &str,
        js_var: &str,
    ) -> Result<bool, WebDriverErrorResponse> {
        let script = format!(
            r"(function() {{
                var parent = window.{parent_js_var};
                if (!parent || !document.contains(parent)) {{
                    throw new Error('stale element reference');
                }}
                var el = {strategy_js};
                if (el) {{
                    window.{js_var} = el;
                    return true;
                }}
                return false;
            }})()"
        );
        let result = self.evaluate_js(&script).await?;
        extract_bool_value(&result)
    }

    /// Find multiple elements from a parent element
    /// Returns count of elements found, stores as {prefix}0, {prefix}1, etc.
    async fn find_elements_from_element(
        &self,
        parent_js_var: &str,
        strategy_js: &str,
        js_var_prefix: &str,
    ) -> Result<usize, WebDriverErrorResponse> {
        let script = format!(
            r"(function() {{
                var parent = window.{parent_js_var};
                if (!parent || !document.contains(parent)) {{
                    throw new Error('stale element reference');
                }}
                var elements = {strategy_js};
                var count = elements.length;
                for (var i = 0; i < count; i++) {{
                    window['{js_var_prefix}' + i] = elements[i];
                }}
                return count;
            }})()"
        );
        let result = self.evaluate_js(&script).await?;
        extract_usize_value(&result)
    }

    /// Get element text content
    async fn get_element_text(&self, js_var: &str) -> Result<String, WebDriverErrorResponse> {
        let script = format!(
            r"(function() {{
                var el = window.{js_var};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                return el.textContent || '';
            }})()"
        );
        let result = self.evaluate_js(&script).await?;
        extract_string_value(&result)
    }

    /// Get element tag name (lowercase)
    async fn get_element_tag_name(&self, js_var: &str) -> Result<String, WebDriverErrorResponse> {
        let script = format!(
            r"(function() {{
                var el = window.{js_var};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                return el.tagName.toLowerCase();
            }})()"
        );
        let result = self.evaluate_js(&script).await?;
        extract_string_value(&result)
    }

    /// Get element attribute value
    async fn get_element_attribute(
        &self,
        js_var: &str,
        name: &str,
    ) -> Result<Option<String>, WebDriverErrorResponse> {
        let escaped_name = name.replace('\\', "\\\\").replace('\'', "\\'");
        let script = format!(
            r"(function() {{
                var el = window.{js_var};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                return el.getAttribute('{escaped_name}');
            }})()"
        );
        let result = self.evaluate_js(&script).await?;

        if let Some(value) = result.get("value") {
            if value.is_null() {
                return Ok(None);
            }
            if let Some(s) = value.as_str() {
                return Ok(Some(s.to_string()));
            }
        }
        Ok(None)
    }

    /// Get element property value
    async fn get_element_property(
        &self,
        js_var: &str,
        name: &str,
    ) -> Result<Value, WebDriverErrorResponse> {
        let escaped_name = name.replace('\\', "\\\\").replace('\'', "\\'");
        let script = format!(
            r"(function() {{
                var el = window.{js_var};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                return el['{escaped_name}'];
            }})()"
        );
        let result = self.evaluate_js(&script).await?;
        extract_value(&result)
    }

    /// Get element CSS property value
    async fn get_element_css_value(
        &self,
        js_var: &str,
        property: &str,
    ) -> Result<String, WebDriverErrorResponse> {
        let escaped_prop = property.replace('\\', "\\\\").replace('\'', "\\'");
        let script = format!(
            r"(function() {{
                var el = window.{js_var};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                return window.getComputedStyle(el).getPropertyValue('{escaped_prop}');
            }})()"
        );
        let result = self.evaluate_js(&script).await?;
        extract_string_value(&result)
    }

    /// Get element bounding rectangle
    async fn get_element_rect(&self, js_var: &str) -> Result<ElementRect, WebDriverErrorResponse> {
        let script = format!(
            r"(function() {{
                var el = window.{js_var};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                var rect = el.getBoundingClientRect();
                return {{
                    x: rect.x + window.scrollX,
                    y: rect.y + window.scrollY,
                    width: rect.width,
                    height: rect.height
                }};
            }})()"
        );
        let result = self.evaluate_js(&script).await?;

        if let Some(value) = result.get("value") {
            return Ok(ElementRect {
                x: value.get("x").and_then(Value::as_f64).unwrap_or(0.0),
                y: value.get("y").and_then(Value::as_f64).unwrap_or(0.0),
                width: value.get("width").and_then(Value::as_f64).unwrap_or(0.0),
                height: value.get("height").and_then(Value::as_f64).unwrap_or(0.0),
            });
        }
        Ok(ElementRect::default())
    }

    /// Check if element is displayed
    async fn is_element_displayed(&self, js_var: &str) -> Result<bool, WebDriverErrorResponse> {
        let script = format!(
            r"(function() {{
                var el = window.{js_var};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                var style = window.getComputedStyle(el);
                return style.display !== 'none' && style.visibility !== 'hidden' && el.offsetParent !== null;
            }})()"
        );
        let result = self.evaluate_js(&script).await?;
        extract_bool_value(&result)
    }

    /// Check if element is enabled
    async fn is_element_enabled(&self, js_var: &str) -> Result<bool, WebDriverErrorResponse> {
        let script = format!(
            r"(function() {{
                var el = window.{js_var};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                return !el.disabled;
            }})()"
        );
        let result = self.evaluate_js(&script).await?;
        extract_bool_value(&result)
    }

    /// Check if element is selected (for checkboxes, radio buttons, options)
    async fn is_element_selected(&self, js_var: &str) -> Result<bool, WebDriverErrorResponse> {
        let script = format!(
            r"(function() {{
                var el = window.{js_var};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                if (el.tagName === 'INPUT' && (el.type === 'checkbox' || el.type === 'radio')) {{
                    return el.checked;
                }}
                if (el.tagName === 'OPTION') {{
                    return el.selected;
                }}
                return false;
            }})()"
        );
        let result = self.evaluate_js(&script).await?;
        extract_bool_value(&result)
    }

    /// Click on element
    async fn click_element(&self, js_var: &str) -> Result<(), WebDriverErrorResponse> {
        let script = format!(
            r"(function() {{
                var el = window.{js_var};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                el.scrollIntoView({{ block: 'center', inline: 'center' }});
                el.click();
                return true;
            }})()"
        );
        self.evaluate_js(&script).await?;
        Ok(())
    }

    /// Clear element content (for inputs/textareas)
    async fn clear_element(&self, js_var: &str) -> Result<(), WebDriverErrorResponse> {
        let script = format!(
            r"(function() {{
                var el = window.{js_var};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                el.focus();
                if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') {{
                    var nativeInputValueSetter = Object.getOwnPropertyDescriptor(
                        el.tagName === 'INPUT' ? window.HTMLInputElement.prototype : window.HTMLTextAreaElement.prototype,
                        'value'
                    ).set;
                    nativeInputValueSetter.call(el, '');
                    var inputEvent = new InputEvent('input', {{
                        bubbles: true,
                        cancelable: true,
                        inputType: 'deleteContentBackward'
                    }});
                    el.dispatchEvent(inputEvent);
                    var changeEvent = new Event('change', {{ bubbles: true }});
                    el.dispatchEvent(changeEvent);
                }} else if (el.isContentEditable) {{
                    el.innerHTML = '';
                }}
                return true;
            }})()"
        );
        self.evaluate_js(&script).await?;
        Ok(())
    }

    /// Send keys to element
    async fn send_keys_to_element(
        &self,
        js_var: &str,
        text: &str,
    ) -> Result<(), WebDriverErrorResponse> {
        let escaped = text
            .replace('\\', "\\\\")
            .replace('`', "\\`")
            .replace('$', "\\$");
        let script = format!(
            r"(function() {{
                var el = window.{js_var};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                el.focus();

                if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') {{
                    var nativeInputValueSetter = Object.getOwnPropertyDescriptor(
                        el.tagName === 'INPUT' ? window.HTMLInputElement.prototype : window.HTMLTextAreaElement.prototype,
                        'value'
                    ).set;

                    var newValue = el.value + `{escaped}`;
                    nativeInputValueSetter.call(el, newValue);

                    var inputEvent = new InputEvent('input', {{
                        bubbles: true,
                        cancelable: true,
                        inputType: 'insertText',
                        data: `{escaped}`
                    }});
                    el.dispatchEvent(inputEvent);

                    var changeEvent = new Event('change', {{ bubbles: true }});
                    el.dispatchEvent(changeEvent);
                }} else if (el.isContentEditable) {{
                    document.execCommand('insertText', false, `{escaped}`);
                }}
                return true;
            }})()"
        );
        self.evaluate_js(&script).await?;
        Ok(())
    }

    /// Get the active (focused) element and store in `js_var`
    /// Returns true if an active element was found
    async fn get_active_element(&self, js_var: &str) -> Result<bool, WebDriverErrorResponse> {
        let script = format!(
            r"(function() {{
                var el = document.activeElement;
                if (el && el !== document.body) {{
                    window.{js_var} = el;
                    return true;
                }}
                return false;
            }})()"
        );
        let result = self.evaluate_js(&script).await?;
        extract_bool_value(&result)
    }

    /// Get element's computed accessibility role
    async fn get_element_computed_role(
        &self,
        js_var: &str,
    ) -> Result<String, WebDriverErrorResponse> {
        let script = format!(
            r"(function() {{
                var el = window.{js_var};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                return el.computedRole || el.getAttribute('role') || '';
            }})()"
        );
        let result = self.evaluate_js(&script).await?;
        extract_string_value(&result)
    }

    /// Get element's computed accessibility label
    async fn get_element_computed_label(
        &self,
        js_var: &str,
    ) -> Result<String, WebDriverErrorResponse> {
        let script = format!(
            r"(function() {{
                var el = window.{js_var};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                return el.computedName || el.getAttribute('aria-label') || el.innerText || '';
            }})()"
        );
        let result = self.evaluate_js(&script).await?;
        extract_string_value(&result)
    }

    // =========================================================================
    // Shadow DOM
    // =========================================================================

    /// Get element's shadow root and store in `shadow_var`
    /// Returns true if shadow root exists
    async fn get_element_shadow_root(
        &self,
        js_var: &str,
        shadow_var: &str,
    ) -> Result<bool, WebDriverErrorResponse> {
        let script = format!(
            r"(function() {{
                var el = window.{js_var};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                var shadow = el.shadowRoot;
                if (shadow) {{
                    window.{shadow_var} = shadow;
                    return true;
                }}
                return false;
            }})()"
        );
        let result = self.evaluate_js(&script).await?;
        extract_bool_value(&result)
    }

    /// Find element within a shadow root
    async fn find_element_from_shadow(
        &self,
        shadow_var: &str,
        strategy_js: &str,
        js_var: &str,
    ) -> Result<bool, WebDriverErrorResponse> {
        let script = format!(
            r"(function() {{
                var shadow = window.{shadow_var};
                if (!shadow) {{
                    throw new Error('no such shadow root');
                }}
                var el = {strategy_js};
                if (el) {{
                    window.{js_var} = el;
                    return true;
                }}
                return false;
            }})()"
        );
        let result = self.evaluate_js(&script).await?;
        extract_bool_value(&result)
    }

    /// Find multiple elements within a shadow root
    async fn find_elements_from_shadow(
        &self,
        shadow_var: &str,
        strategy_js: &str,
        js_var_prefix: &str,
    ) -> Result<usize, WebDriverErrorResponse> {
        let script = format!(
            r"(function() {{
                var shadow = window.{shadow_var};
                if (!shadow) {{
                    throw new Error('no such shadow root');
                }}
                var elements = {strategy_js};
                var count = elements.length;
                for (var i = 0; i < count; i++) {{
                    window['{js_var_prefix}' + i] = elements[i];
                }}
                return count;
            }})()"
        );
        let result = self.evaluate_js(&script).await?;
        extract_usize_value(&result)
    }

    // =========================================================================
    // Script Execution
    // =========================================================================

    /// Execute synchronous JavaScript with arguments
    async fn execute_script(
        &self,
        script: &str,
        args: &[Value],
    ) -> Result<Value, WebDriverErrorResponse> {
        let args_json = serde_json::to_string(args)
            .map_err(|e| WebDriverErrorResponse::invalid_argument(&e.to_string()))?;

        let wrapper = format!(
            r"(function() {{
                var ELEMENT_KEY = 'element-6066-11e4-a52e-4f735466cecf';
                function deserializeArg(arg) {{
                    if (arg === null || arg === undefined) return arg;
                    if (Array.isArray(arg)) return arg.map(deserializeArg);
                    if (typeof arg === 'object') {{
                        if (arg[ELEMENT_KEY]) {{
                            var el = window['__wd_el_' + arg[ELEMENT_KEY].replace(/-/g, '')];
                            if (!el) throw new Error('stale element reference');
                            return el;
                        }}
                        var result = {{}};
                        for (var key in arg) {{
                            if (arg.hasOwnProperty(key)) result[key] = deserializeArg(arg[key]);
                        }}
                        return result;
                    }}
                    return arg;
                }}
                var args = {args_json}.map(deserializeArg);
                var fn = function() {{ {script} }};
                return fn.apply(null, args);
            }})()"
        );
        let result = self.evaluate_js(&wrapper).await?;
        extract_value(&result)
    }

    /// Execute asynchronous JavaScript with callback
    async fn execute_async_script(
        &self,
        script: &str,
        args: &[Value],
    ) -> Result<Value, WebDriverErrorResponse> {
        let args_json = serde_json::to_string(args)
            .map_err(|e| WebDriverErrorResponse::invalid_argument(&e.to_string()))?;

        let wrapper = format!(
            r"new Promise(function(resolve, reject) {{
                try {{
                    var ELEMENT_KEY = 'element-6066-11e4-a52e-4f735466cecf';
                    function deserializeArg(arg) {{
                        if (arg === null || arg === undefined) return arg;
                        if (Array.isArray(arg)) return arg.map(deserializeArg);
                        if (typeof arg === 'object') {{
                            if (arg[ELEMENT_KEY]) {{
                                var el = window['__wd_el_' + arg[ELEMENT_KEY].replace(/-/g, '')];
                                if (!el) throw new Error('stale element reference');
                                return el;
                            }}
                            var result = {{}};
                            for (var key in arg) {{
                                if (arg.hasOwnProperty(key)) result[key] = deserializeArg(arg[key]);
                            }}
                            return result;
                        }}
                        return arg;
                    }}
                    var args = {args_json}.map(deserializeArg);
                    args.push(function(result) {{ resolve(result); }});
                    var fn = function() {{ {script} }};
                    fn.apply(null, args);
                }} catch (e) {{
                    reject(e);
                }}
            }})"
        );

        let result = self.evaluate_js(&wrapper).await?;
        extract_value(&result)
    }

    // =========================================================================
    // Screenshots
    // =========================================================================

    /// Take screenshot of the page, returns base64-encoded PNG
    async fn take_screenshot(&self) -> Result<String, WebDriverErrorResponse>;

    /// Take screenshot of a specific element, returns base64-encoded PNG
    async fn take_element_screenshot(&self, js_var: &str)
        -> Result<String, WebDriverErrorResponse>;

    // =========================================================================
    // Actions (Keyboard/Pointer)
    // =========================================================================

    /// Dispatch a keyboard event
    async fn dispatch_key_event(
        &self,
        key: &str,
        is_down: bool,
    ) -> Result<(), WebDriverErrorResponse>;

    /// Dispatch a pointer/mouse event
    async fn dispatch_pointer_event(
        &self,
        event_type: PointerEventType,
        x: i32,
        y: i32,
        button: u32,
    ) -> Result<(), WebDriverErrorResponse> {
        let event_name = match event_type {
            PointerEventType::Down => "mousedown",
            PointerEventType::Up => "mouseup",
            PointerEventType::Move => "mousemove",
        };

        let buttons = if matches!(event_type, PointerEventType::Down) {
            1 << button
        } else {
            0
        };
        let script = format!(
            r"(function() {{
                var el = document.elementFromPoint({x}, {y});
                if (!el) el = document.body;

                var event = new MouseEvent('{event_name}', {{
                    bubbles: true,
                    cancelable: true,
                    clientX: {x},
                    clientY: {y},
                    button: {button},
                    buttons: {buttons}
                }});
                el.dispatchEvent(event);
                return true;
            }})()"
        );

        self.evaluate_js(&script).await?;
        Ok(())
    }

    /// Dispatch a scroll/wheel event
    async fn dispatch_scroll_event(
        &self,
        x: i32,
        y: i32,
        delta_x: i32,
        delta_y: i32,
    ) -> Result<(), WebDriverErrorResponse> {
        let script = format!(
            r"(function() {{
                var el = document.elementFromPoint({x}, {y});
                if (!el) el = document.body;

                var event = new WheelEvent('wheel', {{
                    bubbles: true,
                    cancelable: true,
                    clientX: {x},
                    clientY: {y},
                    deltaX: {delta_x},
                    deltaY: {delta_y},
                    deltaMode: 0
                }});
                el.dispatchEvent(event);

                window.scrollBy({delta_x}, {delta_y});
                return true;
            }})()"
        );

        self.evaluate_js(&script).await?;
        Ok(())
    }

    // =========================================================================
    // Window Management
    // =========================================================================

    /// Get window rectangle (position and size)
    async fn get_window_rect(&self) -> Result<WindowRect, WebDriverErrorResponse>;

    /// Set window rectangle (position and size)
    async fn set_window_rect(&self, rect: WindowRect)
        -> Result<WindowRect, WebDriverErrorResponse>;

    /// Maximize window
    async fn maximize_window(&self) -> Result<WindowRect, WebDriverErrorResponse>;

    /// Minimize window
    async fn minimize_window(&self) -> Result<(), WebDriverErrorResponse>;

    /// Set window to fullscreen
    async fn fullscreen_window(&self) -> Result<WindowRect, WebDriverErrorResponse>;

    // =========================================================================
    // Frames
    // =========================================================================

    /// Switch to a frame by ID (index, element reference, or null for top)
    async fn switch_to_frame(&self, id: FrameId) -> Result<(), WebDriverErrorResponse> {
        match id {
            FrameId::Top => {
                // Switch back to top-level context
                // TODO: This is a no-op for now as we don't track frame context
                Ok(())
            }
            FrameId::Index(index) => {
                let script = format!(
                    r"(function() {{
                        var frames = document.querySelectorAll('iframe, frame');
                        if ({index} >= frames.length) {{
                            throw new Error('no such frame');
                        }}
                        return true;
                    }})()"
                );
                self.evaluate_js(&script).await?;
                Ok(())
            }
            FrameId::Element(js_var) => {
                let script = format!(
                    r"(function() {{
                        var el = window.{js_var};
                        if (!el || !document.contains(el)) {{
                            throw new Error('stale element reference');
                        }}
                        if (el.tagName !== 'IFRAME' && el.tagName !== 'FRAME') {{
                            throw new Error('element is not a frame');
                        }}
                        return true;
                    }})()"
                );
                self.evaluate_js(&script).await?;
                Ok(())
            }
        }
    }

    /// Switch to parent frame
    async fn switch_to_parent_frame(&self) -> Result<(), WebDriverErrorResponse> {
        // TODO: No-op for now - frame context tracking would be needed
        Ok(())
    }

    // =========================================================================
    // Cookies
    // =========================================================================

    /// Get all cookies
    async fn get_all_cookies(&self) -> Result<Vec<Cookie>, WebDriverErrorResponse> {
        let script = r"(function() {
            var cookies = document.cookie.split(';');
            var result = [];
            for (var i = 0; i < cookies.length; i++) {
                var cookie = cookies[i].trim();
                if (cookie) {
                    var eqIndex = cookie.indexOf('=');
                    if (eqIndex > 0) {
                        result.push({
                            name: cookie.substring(0, eqIndex),
                            value: cookie.substring(eqIndex + 1)
                        });
                    }
                }
            }
            return result;
        })()";

        let result = self.evaluate_js(script).await?;

        if let Some(value) = result.get("value") {
            if let Some(arr) = value.as_array() {
                let cookies: Vec<Cookie> = arr
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();
                return Ok(cookies);
            }
        }
        Ok(vec![])
    }

    /// Get a specific cookie by name
    async fn get_cookie(&self, name: &str) -> Result<Option<Cookie>, WebDriverErrorResponse> {
        let cookies = self.get_all_cookies().await?;
        Ok(cookies.into_iter().find(|c| c.name == name))
    }

    /// Add a cookie
    async fn add_cookie(&self, cookie: Cookie) -> Result<(), WebDriverErrorResponse> {
        let mut cookie_str = format!("{}={}", cookie.name, cookie.value);

        if let Some(path) = &cookie.path {
            let _ = write!(cookie_str, "; path={path}");
        }
        if let Some(domain) = &cookie.domain {
            let _ = write!(cookie_str, "; domain={domain}");
        }
        if cookie.secure {
            cookie_str.push_str("; secure");
        }
        if cookie.http_only {
            cookie_str.push_str("; httponly");
        }
        if let Some(expiry) = cookie.expiry {
            let _ = write!(cookie_str, "; expires={expiry}");
        }
        if let Some(same_site) = &cookie.same_site {
            let _ = write!(cookie_str, "; samesite={same_site}");
        }

        let escaped = cookie_str.replace('\'', "\\'");
        let script = format!(r"document.cookie = '{escaped}'; true");
        self.evaluate_js(&script).await?;
        Ok(())
    }

    /// Delete a cookie by name
    async fn delete_cookie(&self, name: &str) -> Result<(), WebDriverErrorResponse> {
        let script = format!(
            r"document.cookie = '{}=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/'; true",
            name.replace('\'', "\\'")
        );
        self.evaluate_js(&script).await?;
        Ok(())
    }

    /// Delete all cookies
    async fn delete_all_cookies(&self) -> Result<(), WebDriverErrorResponse> {
        let cookies = self.get_all_cookies().await?;
        for cookie in cookies {
            self.delete_cookie(&cookie.name).await?;
        }
        Ok(())
    }

    // =========================================================================
    // Alerts
    // =========================================================================

    /// Dismiss the current alert (cancel)
    async fn dismiss_alert(&self) -> Result<(), WebDriverErrorResponse>;

    /// Accept the current alert (OK)
    async fn accept_alert(&self) -> Result<(), WebDriverErrorResponse>;

    /// Get the text of the current alert
    async fn get_alert_text(&self) -> Result<String, WebDriverErrorResponse>;

    /// Send text to the current alert (for prompts)
    async fn send_alert_text(&self, text: &str) -> Result<(), WebDriverErrorResponse>;

    // =========================================================================
    // Print
    // =========================================================================

    /// Print page to PDF, returns base64-encoded PDF
    async fn print_page(&self, options: PrintOptions) -> Result<String, WebDriverErrorResponse>;
}

// =============================================================================
// Data Types
// =============================================================================

/// Element bounding rectangle
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ElementRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Window rectangle (position and size)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Frame identifier for switching frames
#[derive(Debug, Clone)]
pub enum FrameId {
    /// Top-level browsing context (null)
    Top,
    /// Frame by index
    Index(u32),
    /// Frame by element reference (`js_var`)
    Element(String),
}

/// Pointer event type
#[derive(Debug, Clone, Copy)]
pub enum PointerEventType {
    Down,
    Up,
    Move,
}

/// Cookie data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(default)]
    pub secure: bool,
    #[serde(default, rename = "httpOnly")]
    pub http_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "sameSite")]
    pub same_site: Option<String>,
}

/// Print options for PDF generation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrintOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "pageWidth")]
    pub page_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "pageHeight")]
    pub page_height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "marginTop")]
    pub margin_top: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "marginBottom")]
    pub margin_bottom: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "marginLeft")]
    pub margin_left: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "marginRight")]
    pub margin_right: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "shrinkToFit")]
    pub shrink_to_fit: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "pageRanges")]
    pub page_ranges: Option<Vec<String>>,
}

// =============================================================================
// Helper Functions for Default Implementations
// =============================================================================

/// Extract string value from JavaScript result
fn extract_string_value(result: &Value) -> Result<String, WebDriverErrorResponse> {
    if let Some(success) = result.get("success").and_then(Value::as_bool) {
        if success {
            if let Some(value) = result.get("value") {
                if let Some(s) = value.as_str() {
                    return Ok(s.to_string());
                }
                return Ok(value.to_string());
            }
        } else if let Some(error) = result.get("error").and_then(Value::as_str) {
            return Err(WebDriverErrorResponse::javascript_error(error));
        }
    }
    Ok(String::new())
}

/// Extract boolean value from JavaScript result
fn extract_bool_value(result: &Value) -> Result<bool, WebDriverErrorResponse> {
    if let Some(success) = result.get("success").and_then(Value::as_bool) {
        if success {
            if let Some(value) = result.get("value").and_then(Value::as_bool) {
                return Ok(value);
            }
        } else if let Some(error) = result.get("error").and_then(Value::as_str) {
            return Err(WebDriverErrorResponse::javascript_error(error));
        }
    }
    Ok(false)
}

/// Extract usize value from JavaScript result
fn extract_usize_value(result: &Value) -> Result<usize, WebDriverErrorResponse> {
    if let Some(success) = result.get("success").and_then(Value::as_bool) {
        if success {
            if let Some(count) = result.get("value").and_then(Value::as_u64) {
                return Ok(usize::try_from(count).unwrap_or(0));
            }
        } else if let Some(error) = result.get("error").and_then(Value::as_str) {
            return Err(WebDriverErrorResponse::javascript_error(error));
        }
    }
    Ok(0)
}

/// Extract raw Value from JavaScript result
fn extract_value(result: &Value) -> Result<Value, WebDriverErrorResponse> {
    if let Some(success) = result.get("success").and_then(Value::as_bool) {
        if success {
            return Ok(result.get("value").cloned().unwrap_or(Value::Null));
        } else if let Some(error) = result.get("error").and_then(Value::as_str) {
            return Err(WebDriverErrorResponse::javascript_error(error));
        }
    }
    Ok(Value::Null)
}
