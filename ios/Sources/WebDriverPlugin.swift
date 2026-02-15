import SwiftRs
import Tauri
import UIKit
import WebKit

// MARK: - Argument Classes

class EvaluateJsArgs: Decodable {
    let script: String
    var timeoutMs: Int64?
}

class AsyncScriptArgs: Decodable {
    let asyncId: String
    let script: String
    var timeoutMs: Int64?
}

class TouchArgs: Decodable {
    let type: String
    let x: Int
    let y: Int
}

class ScreenshotArgs: Decodable {
    var timeoutMs: Int64?
}

class PrintArgs: Decodable {
    var orientation: String?
    var scale: Double?
    var background: Bool?
    var pageWidth: Double?
    var pageHeight: Double?
    var marginTop: Double?
    var marginBottom: Double?
    var marginLeft: Double?
    var marginRight: Double?
    var shrinkToFit: Bool?
    var pageRanges: [String]?
}

class SendAlertTextArgs: Decodable {
    let promptText: String
}

// MARK: - Pending Alert

class PendingAlert {
    let message: String
    let type: String  // "alert", "confirm", "prompt"
    let defaultText: String?
    var promptInput: String?
    var completionHandler: ((Bool, String?) -> Void)?

    init(message: String, type: String, defaultText: String? = nil, completionHandler: ((Bool, String?) -> Void)? = nil) {
        self.message = message
        self.type = type
        self.defaultText = defaultText
        self.completionHandler = completionHandler
    }
}

// MARK: - Async Script Bridge

class AsyncScriptBridge: NSObject, WKScriptMessageHandler {
    var pendingCallbacks: [String: (Any?) -> Void] = [:]
    private let lock = NSLock()

    func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
        guard let body = message.body as? [String: Any],
              let asyncId = body["asyncId"] as? String else {
            return
        }

        lock.lock()
        let callback = pendingCallbacks.removeValue(forKey: asyncId)
        lock.unlock()

        if let callback = callback {
            let result = body["result"]
            let error = body["error"] as? String
            if let error = error {
                callback(["error": error])
            } else {
                callback(["result": result ?? NSNull()])
            }
        }
    }

    func registerCallback(asyncId: String, callback: @escaping (Any?) -> Void) {
        lock.lock()
        pendingCallbacks[asyncId] = callback
        lock.unlock()
    }

    func removeCallback(asyncId: String) {
        lock.lock()
        pendingCallbacks.removeValue(forKey: asyncId)
        lock.unlock()
    }
}

// MARK: - WebDriver Plugin

class WebDriverPlugin: Plugin, WKUIDelegate {
    private var webView: WKWebView?
    private var pendingAlert: PendingAlert?
    private let alertLock = NSLock()
    private let asyncBridge = AsyncScriptBridge()
    private var originalUIDelegate: WKUIDelegate?

    @objc public override func load(webview: WKWebView) {
        self.webView = webview

        // Store original delegate to forward non-alert calls
        self.originalUIDelegate = webview.uiDelegate

        // Set ourselves as the UI delegate for alert handling
        webview.uiDelegate = self

        // Add script message handler for async script callbacks
        webview.configuration.userContentController.add(asyncBridge, name: "__webdriver_bridge")
    }

    // MARK: - WKUIDelegate (Alert Handling)

    func webView(_ webView: WKWebView, runJavaScriptAlertPanelWithMessage message: String, initiatedByFrame frame: WKFrameInfo, completionHandler: @escaping () -> Void) {
        alertLock.lock()
        pendingAlert = PendingAlert(message: message, type: "alert") { accepted, _ in
            completionHandler()
        }
        alertLock.unlock()
    }

    func webView(_ webView: WKWebView, runJavaScriptConfirmPanelWithMessage message: String, initiatedByFrame frame: WKFrameInfo, completionHandler: @escaping (Bool) -> Void) {
        alertLock.lock()
        pendingAlert = PendingAlert(message: message, type: "confirm") { accepted, _ in
            completionHandler(accepted)
        }
        alertLock.unlock()
    }

    func webView(_ webView: WKWebView, runJavaScriptTextInputPanelWithPrompt prompt: String, defaultText: String?, initiatedByFrame frame: WKFrameInfo, completionHandler: @escaping (String?) -> Void) {
        alertLock.lock()
        pendingAlert = PendingAlert(message: prompt, type: "prompt", defaultText: defaultText) { accepted, text in
            if accepted {
                completionHandler(text ?? defaultText ?? "")
            } else {
                completionHandler(nil)
            }
        }
        alertLock.unlock()
    }

    // MARK: - Commands

    @objc public func evaluateJs(_ invoke: Invoke) {
        guard let args = try? invoke.parseArgs(EvaluateJsArgs.self) else {
            invoke.reject("Failed to parse arguments")
            return
        }

        guard let wv = webView else {
            invoke.reject("WebView not available")
            return
        }

        DispatchQueue.main.async {
            wv.evaluateJavaScript(args.script) { result, error in
                if let error = error {
                    invoke.resolve([
                        "success": false,
                        "error": error.localizedDescription
                    ])
                } else {
                    // Return the result directly - Tauri will handle JSON serialization
                    // We just need to convert to a JSON-compatible format
                    var jsonValue: Any = NSNull()
                    if let result = result {
                        if result is NSNull {
                            jsonValue = NSNull()
                        } else if let str = result as? String {
                            jsonValue = str
                        } else if let num = result as? NSNumber {
                            jsonValue = num
                        } else if let arr = result as? [Any] {
                            jsonValue = arr
                        } else if let dict = result as? [String: Any] {
                            jsonValue = dict
                        } else {
                            // Fallback: convert to string
                            jsonValue = String(describing: result)
                        }
                    }
                    invoke.resolve([
                        "success": true,
                        "value": jsonValue
                    ])
                }
            }
        }
    }

    @objc public func executeAsyncScript(_ invoke: Invoke) {
        guard let args = try? invoke.parseArgs(AsyncScriptArgs.self) else {
            invoke.reject("Failed to parse arguments")
            return
        }

        guard let wv = webView else {
            invoke.reject("WebView not available")
            return
        }

        let asyncId = args.asyncId
        let timeoutMs = args.timeoutMs ?? 30000

        // Register callback
        asyncBridge.registerCallback(asyncId: asyncId) { response in
            if let response = response as? [String: Any] {
                if let error = response["error"] as? String {
                    invoke.resolve([
                        "success": false,
                        "error": error
                    ])
                } else {
                    // Result is already JSON.stringify'd from JavaScript
                    let jsonValue = response["result"] as? String
                    invoke.resolve([
                        "success": true,
                        "value": jsonValue as Any
                    ])
                }
            } else {
                invoke.resolve([
                    "success": true,
                    "value": NSNull()
                ])
            }
        }

        // Wrap script with callback bridge
        let wrappedScript = """
        (function() {
            var __done = function(result, error) {
                window.webkit.messageHandlers.__webdriver_bridge.postMessage({
                    asyncId: '\(asyncId)',
                    result: result !== undefined ? JSON.stringify(result) : null,
                    error: error || null
                });
            };
            try {
                \(args.script)
            } catch (e) {
                __done(null, e.message || String(e));
            }
        })();
        """

        DispatchQueue.main.async {
            wv.evaluateJavaScript(wrappedScript, completionHandler: nil)
        }

        // Set timeout for cleanup
        DispatchQueue.main.asyncAfter(deadline: .now() + .milliseconds(Int(timeoutMs))) { [weak self] in
            self?.asyncBridge.removeCallback(asyncId: asyncId)
        }
    }

    @objc public func takeScreenshot(_ invoke: Invoke) {
        guard let wv = webView else {
            invoke.reject("WebView not available")
            return
        }

        DispatchQueue.main.async {
            let config = WKSnapshotConfiguration()

            wv.takeSnapshot(with: config) { image, error in
                if let error = error {
                    invoke.resolve([
                        "success": false,
                        "error": error.localizedDescription
                    ])
                    return
                }

                guard let image = image,
                      let pngData = image.pngData() else {
                    invoke.resolve([
                        "success": false,
                        "error": "Failed to capture screenshot"
                    ])
                    return
                }

                let base64 = pngData.base64EncodedString()
                invoke.resolve([
                    "success": true,
                    "value": base64
                ])
            }
        }
    }

    @objc public func printToPdf(_ invoke: Invoke) {
        guard let wv = webView else {
            invoke.reject("WebView not available")
            return
        }

        DispatchQueue.main.async {
            let config = WKPDFConfiguration()

            // Parse optional print arguments
            if let args = try? invoke.parseArgs(PrintArgs.self) {
                // Configure page size if provided (in inches, convert to points)
                if let width = args.pageWidth, let height = args.pageHeight {
                    config.rect = CGRect(x: 0, y: 0, width: width * 72, height: height * 72)
                }
            }

            wv.createPDF(configuration: config) { result in
                switch result {
                case .success(let data):
                    let base64 = data.base64EncodedString()
                    invoke.resolve([
                        "success": true,
                        "value": base64
                    ])
                case .failure(let error):
                    invoke.resolve([
                        "success": false,
                        "error": error.localizedDescription
                    ])
                }
            }
        }
    }

    @objc public func dispatchTouch(_ invoke: Invoke) {
        guard let args = try? invoke.parseArgs(TouchArgs.self) else {
            invoke.reject("Failed to parse arguments")
            return
        }

        guard let wv = webView else {
            invoke.reject("WebView not available")
            return
        }

        // Use JavaScript to dispatch touch/pointer events
        // Native UITouch injection is complex and requires private APIs
        let eventType: String
        switch args.type {
        case "down":
            eventType = "pointerdown"
        case "up":
            eventType = "pointerup"
        case "move":
            eventType = "pointermove"
        default:
            invoke.reject("Unknown touch type: \(args.type)")
            return
        }

        let script = """
        (function() {
            var el = document.elementFromPoint(\(args.x), \(args.y));
            if (el) {
                var event = new PointerEvent('\(eventType)', {
                    bubbles: true,
                    cancelable: true,
                    clientX: \(args.x),
                    clientY: \(args.y),
                    pointerId: 1,
                    pointerType: 'touch',
                    isPrimary: true
                });
                el.dispatchEvent(event);
            }
        })();
        """

        DispatchQueue.main.async {
            wv.evaluateJavaScript(script) { _, error in
                if let error = error {
                    invoke.reject("Touch dispatch failed: \(error.localizedDescription)")
                } else {
                    invoke.resolve()
                }
            }
        }
    }

    @objc public func getAlertText(_ invoke: Invoke) {
        alertLock.lock()
        let alert = pendingAlert
        alertLock.unlock()

        if let alert = alert {
            invoke.resolve([
                "message": alert.message,
                "type": alert.type,
                "defaultText": alert.defaultText as Any
            ])
        } else {
            invoke.reject("no such alert")
        }
    }

    @objc public func acceptAlert(_ invoke: Invoke) {
        alertLock.lock()
        let alert = pendingAlert
        pendingAlert = nil
        alertLock.unlock()

        if let alert = alert {
            let promptText = alert.promptInput ?? alert.defaultText
            alert.completionHandler?(true, promptText)
            invoke.resolve()
        } else {
            invoke.reject("no such alert")
        }
    }

    @objc public func dismissAlert(_ invoke: Invoke) {
        alertLock.lock()
        let alert = pendingAlert
        pendingAlert = nil
        alertLock.unlock()

        if let alert = alert {
            alert.completionHandler?(false, nil)
            invoke.resolve()
        } else {
            invoke.reject("no such alert")
        }
    }

    @objc public func sendAlertText(_ invoke: Invoke) {
        guard let args = try? invoke.parseArgs(SendAlertTextArgs.self) else {
            invoke.reject("Failed to parse arguments")
            return
        }

        alertLock.lock()
        let alert = pendingAlert
        alertLock.unlock()

        if let alert = alert {
            if alert.type == "prompt" {
                alert.promptInput = args.promptText
                invoke.resolve()
            } else {
                invoke.reject("Alert is not a prompt")
            }
        } else {
            invoke.reject("no such alert")
        }
    }

    @objc public func getViewportSize(_ invoke: Invoke) {
        guard let wv = webView else {
            invoke.reject("WebView not available")
            return
        }

        DispatchQueue.main.async {
            invoke.resolve([
                "width": Int(wv.bounds.width),
                "height": Int(wv.bounds.height)
            ])
        }
    }
}

@_cdecl("init_plugin_webdriver")
func initPlugin() -> Plugin {
    return WebDriverPlugin()
}
