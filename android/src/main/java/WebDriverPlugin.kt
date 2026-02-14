package com.plugin.webdriver

import android.app.Activity
import android.graphics.Bitmap
import android.os.Handler
import android.os.Looper
import android.view.MotionEvent
import android.webkit.JavascriptInterface
import android.webkit.WebView
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.ByteArrayOutputStream
import java.util.concurrent.ConcurrentHashMap

@InvokeArg
class EvaluateJsArgs {
    lateinit var script: String
    var timeoutMs: Long = 30000
}

@InvokeArg
class ScreenshotArgs {
    var timeoutMs: Long = 30000
}

@InvokeArg
class PrintArgs {
    var orientation: String? = null
    var scale: Double? = null
    var background: Boolean? = null
    var pageWidth: Double? = null
    var pageHeight: Double? = null
    var marginTop: Double? = null
    var marginBottom: Double? = null
    var marginLeft: Double? = null
    var marginRight: Double? = null
    var shrinkToFit: Boolean? = null
    var pageRanges: List<String>? = null
}

@InvokeArg
class TouchArgs {
    lateinit var type: String  // "down", "up", "move"
    var x: Int = 0
    var y: Int = 0
}

@InvokeArg
class AsyncScriptArgs {
    lateinit var asyncId: String
    lateinit var script: String
    var timeoutMs: Long = 30000
}

@InvokeArg
class AlertResponseArgs {
    var accepted: Boolean = true
    var promptText: String? = null
}

data class PendingAlert(
    val message: String,
    val defaultText: String?,
    val type: String, // "alert", "confirm", "prompt"
    var promptInput: String? = null,
    var responseCallback: ((Boolean, String?) -> Unit)? = null
)

@TauriPlugin
class WebDriverPlugin(private val activity: Activity) : Plugin(activity) {
    private var webView: WebView? = null
    private val mainHandler = Handler(Looper.getMainLooper())
    private val pendingAsyncScripts = ConcurrentHashMap<String, (Any?) -> Unit>()
    private var pendingAlert: PendingAlert? = null
    private val alertLock = Object()

    override fun load(webView: WebView) {
        this.webView = webView

        // Add JavaScript interface for async script callbacks
        mainHandler.post {
            webView.addJavascriptInterface(AsyncScriptBridge(), "__webdriver_bridge")
        }
    }

    /**
     * JavaScript interface for async script callbacks
     */
    inner class AsyncScriptBridge {
        @JavascriptInterface
        fun postResult(asyncId: String, result: String?, error: String?) {
            val callback = pendingAsyncScripts.remove(asyncId)
            if (callback != null) {
                if (error != null) {
                    callback(mapOf("error" to error))
                } else {
                    callback(mapOf("result" to result))
                }
            }
        }
    }

    /**
     * Evaluate JavaScript synchronously and return result
     */
    @Command
    fun evaluateJs(invoke: Invoke) {
        val args = invoke.parseArgs(EvaluateJsArgs::class.java)
        val wv = webView

        if (wv == null) {
            invoke.reject("WebView not available")
            return
        }

        mainHandler.post {
            wv.evaluateJavascript(args.script) { result ->
                val ret = JSObject()
                ret.put("success", true)
                ret.put("value", result)
                invoke.resolve(ret)
            }
        }
    }

    /**
     * Execute async script with callback support
     */
    @Command
    fun executeAsyncScript(invoke: Invoke) {
        val args = invoke.parseArgs(AsyncScriptArgs::class.java)
        val wv = webView

        if (wv == null) {
            invoke.reject("WebView not available")
            return
        }

        // Register callback for this async operation
        pendingAsyncScripts[args.asyncId] = { response ->
            val ret = JSObject()
            if (response is Map<*, *>) {
                val error = response["error"] as? String
                val result = response["result"] as? String
                if (error != null) {
                    ret.put("success", false)
                    ret.put("error", error)
                } else {
                    ret.put("success", true)
                    ret.put("value", result)
                }
            } else {
                ret.put("success", true)
                ret.put("value", response)
            }
            invoke.resolve(ret)
        }

        // Inject script with callback bridge
        val wrappedScript = """
            (function() {
                var __done = function(result) {
                    __webdriver_bridge.postResult('${args.asyncId}', JSON.stringify(result), null);
                };
                try {
                    ${args.script}
                } catch (e) {
                    __webdriver_bridge.postResult('${args.asyncId}', null, e.message || String(e));
                }
            })();
        """.trimIndent()

        mainHandler.post {
            wv.evaluateJavascript(wrappedScript, null)
        }

        // Set timeout for cleanup
        mainHandler.postDelayed({
            val callback = pendingAsyncScripts.remove(args.asyncId)
            if (callback != null) {
                val ret = JSObject()
                ret.put("success", false)
                ret.put("error", "Script timeout")
                invoke.resolve(ret)
            }
        }, args.timeoutMs)
    }

    /**
     * Take screenshot by drawing WebView to Canvas
     */
    @Command
    fun takeScreenshot(invoke: Invoke) {
        val wv = webView

        if (wv == null) {
            invoke.reject("WebView not available")
            return
        }

        mainHandler.post {
            try {
                val bitmap = Bitmap.createBitmap(wv.width, wv.height, Bitmap.Config.ARGB_8888)
                val canvas = android.graphics.Canvas(bitmap)
                wv.draw(canvas)

                val outputStream = ByteArrayOutputStream()
                bitmap.compress(Bitmap.CompressFormat.PNG, 100, outputStream)
                val base64 = android.util.Base64.encodeToString(outputStream.toByteArray(), android.util.Base64.NO_WRAP)

                val ret = JSObject()
                ret.put("success", true)
                ret.put("value", base64)
                invoke.resolve(ret)

                bitmap.recycle()
            } catch (e: Exception) {
                invoke.reject("Screenshot failed: ${e.message}")
            }
        }
    }

    /**
     * Print page to PDF
     * Note: Android WebView doesn't have direct PDF export like WKWebView.
     * We use the print adapter approach.
     */
    @Command
    fun printToPdf(invoke: Invoke) {
        val args = invoke.parseArgs(PrintArgs::class.java)
        val wv = webView

        if (wv == null) {
            invoke.reject("WebView not available")
            return
        }

        // Android doesn't support direct PDF generation from WebView without PrintManager
        // For now, return an error - this requires PrintDocumentAdapter which writes to a file
        invoke.reject("Print to PDF not yet implemented on Android. Use PrintManager for printing.")
    }

    /**
     * Dispatch touch event
     */
    @Command
    fun dispatchTouch(invoke: Invoke) {
        val args = invoke.parseArgs(TouchArgs::class.java)
        val wv = webView

        if (wv == null) {
            invoke.reject("WebView not available")
            return
        }

        mainHandler.post {
            try {
                val action = when (args.type) {
                    "down" -> MotionEvent.ACTION_DOWN
                    "up" -> MotionEvent.ACTION_UP
                    "move" -> MotionEvent.ACTION_MOVE
                    else -> {
                        invoke.reject("Unknown touch type: ${args.type}")
                        return@post
                    }
                }

                val downTime = System.currentTimeMillis()
                val eventTime = System.currentTimeMillis()

                val event = MotionEvent.obtain(
                    downTime,
                    eventTime,
                    action,
                    args.x.toFloat(),
                    args.y.toFloat(),
                    0
                )

                wv.dispatchTouchEvent(event)
                event.recycle()

                invoke.resolve()
            } catch (e: Exception) {
                invoke.reject("Touch dispatch failed: ${e.message}")
            }
        }
    }

    /**
     * Get current alert text (if any alert is pending)
     */
    @Command
    fun getAlertText(invoke: Invoke) {
        synchronized(alertLock) {
            val alert = pendingAlert
            if (alert != null) {
                val ret = JSObject()
                ret.put("message", alert.message)
                ret.put("type", alert.type)
                ret.put("defaultText", alert.defaultText)
                invoke.resolve(ret)
            } else {
                invoke.reject("no such alert")
            }
        }
    }

    /**
     * Accept current alert
     */
    @Command
    fun acceptAlert(invoke: Invoke) {
        synchronized(alertLock) {
            val alert = pendingAlert
            if (alert != null) {
                val promptText = alert.promptInput ?: alert.defaultText
                alert.responseCallback?.invoke(true, promptText)
                pendingAlert = null
                invoke.resolve()
            } else {
                invoke.reject("no such alert")
            }
        }
    }

    /**
     * Dismiss current alert
     */
    @Command
    fun dismissAlert(invoke: Invoke) {
        synchronized(alertLock) {
            val alert = pendingAlert
            if (alert != null) {
                alert.responseCallback?.invoke(false, null)
                pendingAlert = null
                invoke.resolve()
            } else {
                invoke.reject("no such alert")
            }
        }
    }

    /**
     * Send text to prompt dialog
     */
    @Command
    fun sendAlertText(invoke: Invoke) {
        val args = invoke.parseArgs(AlertResponseArgs::class.java)
        synchronized(alertLock) {
            val alert = pendingAlert
            if (alert != null) {
                if (alert.type == "prompt") {
                    alert.promptInput = args.promptText
                    invoke.resolve()
                } else {
                    invoke.reject("Alert is not a prompt")
                }
            } else {
                invoke.reject("no such alert")
            }
        }
    }

    /**
     * Internal method to set pending alert (called from WebChromeClient)
     */
    fun setPendingAlert(
        message: String,
        defaultText: String?,
        type: String,
        callback: (Boolean, String?) -> Unit
    ) {
        synchronized(alertLock) {
            pendingAlert = PendingAlert(
                message = message,
                defaultText = defaultText,
                type = type,
                responseCallback = callback
            )
        }
    }

    /**
     * Get WebView dimensions
     */
    @Command
    fun getViewportSize(invoke: Invoke) {
        val wv = webView

        if (wv == null) {
            invoke.reject("WebView not available")
            return
        }

        mainHandler.post {
            val ret = JSObject()
            ret.put("width", wv.width)
            ret.put("height", wv.height)
            invoke.resolve(ret)
        }
    }
}
