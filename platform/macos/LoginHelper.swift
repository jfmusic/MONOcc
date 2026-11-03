import AppKit

final class LoginDelegate:NSObject,NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification:Notification) {
        if NSRunningApplication.runningApplications(withBundleIdentifier:"org.monocc.studio").contains(where:{!$0.isTerminated}) {NSApp.terminate(nil);return}
        var appURL=Bundle.main.bundleURL
        for _ in 0..<4 {appURL.deleteLastPathComponent()}
        guard Bundle(url:appURL)?.bundleIdentifier == "org.monocc.studio" else {NSApp.terminate(nil);return}
        let configuration=NSWorkspace.OpenConfiguration()
        configuration.activates=false;configuration.arguments=["--background"]
        NSWorkspace.shared.openApplication(at:appURL,configuration:configuration){_,_ in DispatchQueue.main.async{NSApp.terminate(nil)}}
    }
}
@main struct MONOccLogin {
    static func main() {
        let app=NSApplication.shared;app.setActivationPolicy(.accessory)
        let delegate=LoginDelegate();app.delegate=delegate
        withExtendedLifetime(delegate){app.run()}
    }
}
