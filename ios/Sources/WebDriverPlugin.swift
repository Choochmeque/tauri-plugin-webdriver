import SwiftRs
import Tauri
import UIKit
import WebKit

class WebDriverPlugin: Plugin {

}

@_cdecl("init_plugin_webdriver")
func initPlugin() -> Plugin {
  return WebDriverPlugin()
}
