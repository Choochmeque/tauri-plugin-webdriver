use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tauri::{Runtime, WebviewWindow};
use tokio::sync::oneshot;
use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2ExecuteScriptCompletedHandler;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};

use crate::platform::{
    Cookie, FrameId, PlatformExecutor, PointerEventType, PrintOptions, WindowRect,
};
use crate::server::response::WebDriverErrorResponse;
use crate::webdriver::Timeouts;

/// Windows `WebView2` executor
#[derive(Clone)]
pub struct WindowsExecutor<R: Runtime> {
    window: WebviewWindow<R>,
    timeouts: Timeouts,
}

impl<R: Runtime> WindowsExecutor<R> {
    pub fn new(window: WebviewWindow<R>, timeouts: Timeouts) -> Self {
        Self { window, timeouts }
    }
}

#[async_trait]
impl<R: Runtime + 'static> PlatformExecutor for WindowsExecutor<R> {
    // =========================================================================
    // Core JavaScript Execution
    // =========================================================================

    async fn evaluate_js(&self, script: &str) -> Result<Value, WebDriverErrorResponse> {
        let (tx, rx) = oneshot::channel();
        let script_owned = script.to_string();

        let result = self.window.with_webview(move |webview| unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

            if let Ok(webview2) = webview.controller().CoreWebView2() {
                let script_hstring = HSTRING::from(&script_owned);

                let tx = Arc::new(std::sync::Mutex::new(Some(tx)));
                let handler: ICoreWebView2ExecuteScriptCompletedHandler =
                    ExecuteScriptHandler::new(tx).into();

                webview2
                    .ExecuteScript(PCWSTR(script_hstring.as_ptr()), &handler)
                    .ok();
            }
        });

        if let Err(e) = result {
            return Err(WebDriverErrorResponse::javascript_error(&e.to_string()));
        }

        let timeout = std::time::Duration::from_millis(self.timeouts.script_ms);
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(value))) => Ok(serde_json::json!({
                "success": true,
                "value": value
            })),
            Ok(Ok(Err(error))) => Err(WebDriverErrorResponse::javascript_error(&error)),
            Ok(Err(_)) => Err(WebDriverErrorResponse::javascript_error("Channel closed")),
            Err(_) => Err(WebDriverErrorResponse::script_timeout()),
        }
    }

    // =========================================================================
    // Document
    // =========================================================================

    async fn get_source(&self) -> Result<String, WebDriverErrorResponse> {
        let result = self
            .evaluate_js("document.documentElement.outerHTML")
            .await?;
        extract_string_value(&result)
    }

    // =========================================================================
    // Element Operations
    // =========================================================================

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

        if let Some(success) = result.get("success").and_then(Value::as_bool) {
            if success {
                if let Some(count) = result.get("value").and_then(Value::as_u64) {
                    return Ok(usize::try_from(count).unwrap_or(0));
                }
            }
        }
        Ok(0)
    }

    // =========================================================================
    // Script Execution
    // =========================================================================

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

        if let Some(success) = result.get("success").and_then(Value::as_bool) {
            if success {
                return Ok(result.get("value").cloned().unwrap_or(Value::Null));
            } else if let Some(error) = result.get("error").and_then(Value::as_str) {
                return Err(WebDriverErrorResponse::javascript_error(error));
            }
        }

        Ok(Value::Null)
    }

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

        if let Some(success) = result.get("success").and_then(Value::as_bool) {
            if success {
                return Ok(result.get("value").cloned().unwrap_or(Value::Null));
            } else if let Some(error) = result.get("error").and_then(Value::as_str) {
                return Err(WebDriverErrorResponse::javascript_error(error));
            }
        }

        Ok(Value::Null)
    }

    // =========================================================================
    // Screenshots
    // =========================================================================

    async fn take_screenshot(&self) -> Result<String, WebDriverErrorResponse> {
        // Use JavaScript canvas-based screenshot for cross-platform compatibility
        let _script = r"(function() {
            return new Promise(function(resolve, reject) {
                try {
                    var canvas = document.createElement('canvas');
                    var ctx = canvas.getContext('2d');
                    canvas.width = document.documentElement.scrollWidth;
                    canvas.height = document.documentElement.scrollHeight;

                    // For simple pages, we can use html2canvas-like approach
                    // TODO: For now, return a placeholder - native WebView2 CapturePreview would be better
                    resolve('');
                } catch (e) {
                    reject(e.message);
                }
            });
        })()";

        // For Windows, we should use WebView2's CapturePreview API
        // This is a simplified implementation - full implementation would use native API
        let (tx, rx) = oneshot::channel();

        let result = self.window.with_webview(move |webview| {
            unsafe {
                if let Ok(_webview2) = webview.controller().CoreWebView2() {
                    // Create a memory stream for the image
                    // Note: This requires additional COM setup for IStream
                    // For now, return a placeholder

                    let tx = Arc::new(std::sync::Mutex::new(Some(tx)));
                    let handler = CapturePreviewHandler::new(tx);

                    // TODO: CapturePreview requires an IStream - simplified for now
                    // webview2.CapturePreview(COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG, stream, &handler);

                    // For now, signal completion with empty result
                    if let Ok(mut guard) = handler.tx.lock() {
                        if let Some(tx) = guard.take() {
                            let _ = tx.send(Ok(String::new()));
                        }
                    };
                }
            }
        });

        if let Err(e) = result {
            return Err(WebDriverErrorResponse::unknown_error(&e.to_string()));
        }

        let timeout = std::time::Duration::from_millis(self.timeouts.script_ms);
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(base64))) => {
                if base64.is_empty() {
                    // Fallback to JS-based screenshot
                    self.take_js_screenshot().await
                } else {
                    Ok(base64)
                }
            }
            Ok(Ok(Err(error))) => Err(WebDriverErrorResponse::unknown_error(&error)),
            Ok(Err(_)) => Err(WebDriverErrorResponse::unknown_error("Channel closed")),
            Err(_) => Err(WebDriverErrorResponse::script_timeout()),
        }
    }

    async fn take_element_screenshot(
        &self,
        js_var: &str,
    ) -> Result<String, WebDriverErrorResponse> {
        // Scroll element into view first
        let script = format!(
            r"(function() {{
                var el = window.{js_var};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                el.scrollIntoView({{ block: 'center', inline: 'center' }});
                return true;
            }})()"
        );
        self.evaluate_js(&script).await?;

        // Take full screenshot and return (element clipping can be added later)
        self.take_screenshot().await
    }

    // =========================================================================
    // Actions (Keyboard/Pointer)
    // =========================================================================

    async fn dispatch_key_event(
        &self,
        key: &str,
        is_down: bool,
    ) -> Result<(), WebDriverErrorResponse> {
        let (js_key, js_code, key_code) = match key {
            "\u{E007}" => ("Enter", "Enter", 13),
            "\u{E003}" => ("Backspace", "Backspace", 8),
            "\u{E004}" => ("Tab", "Tab", 9),
            "\u{E006}" => ("Enter", "NumpadEnter", 13),
            "\u{E00C}" => ("Escape", "Escape", 27),
            "\u{E00D}" => (" ", "Space", 32),
            "\u{E012}" => ("ArrowLeft", "ArrowLeft", 37),
            "\u{E013}" => ("ArrowUp", "ArrowUp", 38),
            "\u{E014}" => ("ArrowRight", "ArrowRight", 39),
            "\u{E015}" => ("ArrowDown", "ArrowDown", 40),
            "\u{E017}" => ("Delete", "Delete", 46),
            "\u{E008}" => ("Shift", "ShiftLeft", 16),
            "\u{E009}" => ("Control", "ControlLeft", 17),
            "\u{E00A}" => ("Alt", "AltLeft", 18),
            "\u{E03D}" => ("Meta", "MetaLeft", 91),
            _ => {
                let ch = key.chars().next().unwrap_or(' ');
                let upper = ch.to_ascii_uppercase();
                let code = if ch.is_ascii_alphabetic() {
                    format!("Key{upper}")
                } else if ch.is_ascii_digit() {
                    format!("Digit{ch}")
                } else {
                    key.to_string()
                };
                return self.dispatch_regular_key(key, &code, is_down).await;
            }
        };

        let event_type = if is_down { "keydown" } else { "keyup" };
        let script = format!(
            r"(function() {{
                var event = new KeyboardEvent('{event_type}', {{
                    key: '{js_key}',
                    code: '{js_code}',
                    keyCode: {key_code},
                    which: {key_code},
                    bubbles: true,
                    cancelable: true
                }});
                var activeEl = document.activeElement || document.body;
                activeEl.dispatchEvent(event);
                return true;
            }})()"
        );

        self.evaluate_js(&script).await?;
        Ok(())
    }

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

                // Also perform actual scroll
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

    async fn get_window_rect(&self) -> Result<WindowRect, WebDriverErrorResponse> {
        if let Ok(position) = self.window.outer_position() {
            if let Ok(size) = self.window.outer_size() {
                return Ok(WindowRect {
                    x: position.x,
                    y: position.y,
                    width: size.width,
                    height: size.height,
                });
            }
        }
        Ok(WindowRect::default())
    }

    async fn set_window_rect(
        &self,
        rect: WindowRect,
    ) -> Result<WindowRect, WebDriverErrorResponse> {
        use tauri::{PhysicalPosition, PhysicalSize};

        let _ = self
            .window
            .set_position(PhysicalPosition::new(rect.x, rect.y));
        let _ = self
            .window
            .set_size(PhysicalSize::new(rect.width, rect.height));

        self.get_window_rect().await
    }

    async fn maximize_window(&self) -> Result<WindowRect, WebDriverErrorResponse> {
        let _ = self.window.maximize();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        self.get_window_rect().await
    }

    async fn minimize_window(&self) -> Result<(), WebDriverErrorResponse> {
        let _ = self.window.minimize();
        Ok(())
    }

    async fn fullscreen_window(&self) -> Result<WindowRect, WebDriverErrorResponse> {
        let _ = self.window.set_fullscreen(true);
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        self.get_window_rect().await
    }

    // =========================================================================
    // Frames
    // =========================================================================

    async fn switch_to_frame(&self, id: FrameId) -> Result<(), WebDriverErrorResponse> {
        match id {
            FrameId::Top => Ok(()),
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

    async fn switch_to_parent_frame(&self) -> Result<(), WebDriverErrorResponse> {
        Ok(())
    }

    // =========================================================================
    // Cookies
    // =========================================================================

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

    async fn get_cookie(&self, name: &str) -> Result<Option<Cookie>, WebDriverErrorResponse> {
        let cookies = self.get_all_cookies().await?;
        Ok(cookies.into_iter().find(|c| c.name == name))
    }

    async fn add_cookie(&self, cookie: Cookie) -> Result<(), WebDriverErrorResponse> {
        use std::fmt::Write;

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

    async fn delete_cookie(&self, name: &str) -> Result<(), WebDriverErrorResponse> {
        let script = format!(
            r"document.cookie = '{}=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/'; true",
            name.replace('\'', "\\'")
        );
        self.evaluate_js(&script).await?;
        Ok(())
    }

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

    async fn dismiss_alert(&self) -> Result<(), WebDriverErrorResponse> {
        Err(WebDriverErrorResponse::unsupported_operation(
            "Alert handling not yet implemented for Windows",
        ))
    }

    async fn accept_alert(&self) -> Result<(), WebDriverErrorResponse> {
        Err(WebDriverErrorResponse::unsupported_operation(
            "Alert handling not yet implemented for Windows",
        ))
    }

    async fn get_alert_text(&self) -> Result<String, WebDriverErrorResponse> {
        Err(WebDriverErrorResponse::unsupported_operation(
            "Alert handling not yet implemented for Windows",
        ))
    }

    async fn send_alert_text(&self, _text: &str) -> Result<(), WebDriverErrorResponse> {
        Err(WebDriverErrorResponse::unsupported_operation(
            "Alert handling not yet implemented for Windows",
        ))
    }

    // =========================================================================
    // Print
    // =========================================================================

    async fn print_page(&self, _options: PrintOptions) -> Result<String, WebDriverErrorResponse> {
        Err(WebDriverErrorResponse::unsupported_operation(
            "PDF printing not yet implemented for Windows",
        ))
    }
}

// =============================================================================
// Helper Methods
// =============================================================================

impl<R: Runtime + 'static> WindowsExecutor<R> {
    async fn dispatch_regular_key(
        &self,
        key: &str,
        code: &str,
        is_down: bool,
    ) -> Result<(), WebDriverErrorResponse> {
        let ch = key.chars().next().unwrap_or(' ');
        let key_code = ch as u32;
        let event_type = if is_down { "keydown" } else { "keyup" };

        let escaped_key = key.replace('\\', "\\\\").replace('\'', "\\'");
        let escaped_code = code.replace('\\', "\\\\").replace('\'', "\\'");

        let script = format!(
            r"(function() {{
                var event = new KeyboardEvent('{event_type}', {{
                    key: '{escaped_key}',
                    code: '{escaped_code}',
                    keyCode: {key_code},
                    which: {key_code},
                    bubbles: true,
                    cancelable: true
                }});
                var activeEl = document.activeElement || document.body;
                activeEl.dispatchEvent(event);
                return true;
            }})()"
        );

        self.evaluate_js(&script).await?;
        Ok(())
    }

    async fn take_js_screenshot(&self) -> Result<String, WebDriverErrorResponse> {
        // JavaScript-based screenshot using canvas
        let script = r"(function() {
            return new Promise(function(resolve, reject) {
                try {
                    // This is a simplified approach - full implementation would use html2canvas
                    var canvas = document.createElement('canvas');
                    var ctx = canvas.getContext('2d');
                    canvas.width = window.innerWidth;
                    canvas.height = window.innerHeight;

                    // Draw a white background
                    ctx.fillStyle = 'white';
                    ctx.fillRect(0, 0, canvas.width, canvas.height);

                    // For now, return base64 of blank canvas
                    // Full implementation would render DOM to canvas
                    var dataUrl = canvas.toDataURL('image/png');
                    resolve(dataUrl.replace('data:image/png;base64,', ''));
                } catch (e) {
                    reject(e.message);
                }
            });
        })()";

        let result = self.evaluate_js(script).await?;
        extract_string_value(&result)
    }
}

// =============================================================================
// COM Handlers
// =============================================================================

type ScriptResultSender = Arc<std::sync::Mutex<Option<oneshot::Sender<Result<Value, String>>>>>;
type CaptureResultSender = Arc<std::sync::Mutex<Option<oneshot::Sender<Result<String, String>>>>>;

mod handlers {
    #![allow(clippy::inline_always, clippy::ref_as_ptr)]

    use serde_json::Value;
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2CapturePreviewCompletedHandler,
        ICoreWebView2CapturePreviewCompletedHandler_Impl,
        ICoreWebView2ExecuteScriptCompletedHandler,
        ICoreWebView2ExecuteScriptCompletedHandler_Impl,
    };
    use windows::core::implement;

    use super::{CaptureResultSender, ScriptResultSender};

    #[implement(ICoreWebView2ExecuteScriptCompletedHandler)]
    pub struct ExecuteScriptHandler {
        pub tx: ScriptResultSender,
    }

    impl ExecuteScriptHandler {
        pub fn new(tx: ScriptResultSender) -> Self {
            Self { tx }
        }
    }

    impl ICoreWebView2ExecuteScriptCompletedHandler_Impl for ExecuteScriptHandler_Impl {
        fn Invoke(
            &self,
            errorcode: windows::core::HRESULT,
            resultobjectasjson: &windows::core::PCWSTR,
        ) -> windows::core::Result<()> {
            let response = if errorcode.is_err() {
                Err(format!("Script execution failed: {errorcode:?}"))
            } else {
                let json_str = unsafe { resultobjectasjson.to_string().unwrap_or_default() };
                match serde_json::from_str(&json_str) {
                    Ok(value) => Ok(value),
                    Err(_) => Ok(Value::String(json_str)),
                }
            };

            if let Ok(mut guard) = self.tx.lock() {
                if let Some(tx) = guard.take() {
                    let _ = tx.send(response);
                }
            }
            Ok(())
        }
    }

    #[implement(ICoreWebView2CapturePreviewCompletedHandler)]
    pub struct CapturePreviewHandler {
        pub tx: CaptureResultSender,
    }

    impl CapturePreviewHandler {
        pub fn new(tx: CaptureResultSender) -> Self {
            Self { tx }
        }
    }

    impl ICoreWebView2CapturePreviewCompletedHandler_Impl for CapturePreviewHandler_Impl {
        fn Invoke(&self, errorcode: windows::core::HRESULT) -> windows::core::Result<()> {
            let response = if errorcode.is_err() {
                Err(format!("Capture preview failed: {errorcode:?}"))
            } else {
                // In a full implementation, we'd read the IStream here
                Ok(String::new())
            };

            if let Ok(mut guard) = self.tx.lock() {
                if let Some(tx) = guard.take() {
                    let _ = tx.send(response);
                }
            }
            Ok(())
        }
    }
}

use handlers::{CapturePreviewHandler, ExecuteScriptHandler};

// =============================================================================
// Utility Functions
// =============================================================================

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
