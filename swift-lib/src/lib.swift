import SwiftRs
import AppKit
import ApplicationServices
import Cocoa

let CGS_CONNECTION = CGSMainConnectionID()
typealias CGSConnectionID = UInt32
typealias CGSSpaceID = UInt64

@_silgen_name("CGSCopyWindowsWithOptionsAndTags")
func CGSCopyWindowsWithOptionsAndTags(_ cid: CGSConnectionID, _ owner: Int, _ spaces: CFArray, _ options: Int, _ setTags: inout Int, _ clearTags: inout Int) -> CFArray

@_silgen_name("CGSCopyManagedDisplaySpaces")
func CGSCopyManagedDisplaySpaces(_ cid: CGSConnectionID) -> CFArray

@_silgen_name("CGSMainConnectionID")
func CGSMainConnectionID() -> CGSConnectionID

class Application: NSObject {
    var name: SRString
    var icon_path: SRString
    var pid: Int

    public init(name: String, icon_path: String, pid: Int) {
        self.name = SRString(name);
        self.icon_path = SRString(icon_path);
        self.pid = pid;
    }
}

@_cdecl("enable_accessibility_features")
func enableAccessibilityFeatures() -> Bool {
    let options: NSDictionary = [kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: true]
    let accessibilityEnabled = AXIsProcessTrustedWithOptions(options)

    if accessibilityEnabled {
        print("Accessibility features enabled.")
    } else {
        print("Accessibility features not enabled. Please enable in System Preferences.")
    }

    return accessibilityEnabled
}

@_cdecl("get_active_app")
func getActiveApp() -> SRString {
    let runningApps = NSWorkspace.shared.runningApplications

    guard let app = runningApps.first(where: { $0.isActive }) else {
        print("No active application.")
        return SRString("")
    }

    if let localizedName = app.localizedName {
        return SRString(localizedName)
    }

    return SRString("")
}

@_cdecl("fire_window_event")
func fireWindowEvent(pid: Int, type: SRString) {
    let runningApps = NSWorkspace.shared.runningApplications

    guard let app = runningApps.first(where: { Int($0.processIdentifier) == pid }) else {
        print("Application \(pid) is not running.")
        return
    }

    switch type.toString() {
    case "minimize":
        app.hide()
    case "quit":
        app.terminate()
    default:
        // By default, activation brings only the main and key windows forward
        // - https://developer.apple.com/documentation/appkit/nsapplication/activationoptions?changes=_9
        app.activate()
    }
}

@_cdecl("get_application_windows")
func getApplicationWindows() -> SRObjectArray {
    var result: [Application] = []
    let runningApps = NSWorkspace.shared.runningApplications

    var windowPidMap = [Int: Int]()
    (CGWindowListCopyWindowInfo([.excludeDesktopElements, .optionAll], 0) as! [[String: Any]]).forEach { (item: [String: Any]) in
        if let windowNumber = item[kCGWindowNumber as String] as? Int,
           let ownerPID = item[kCGWindowOwnerPID as String] as? Int {
            windowPidMap[windowNumber] = ownerPID
        }
    }

    var windowLevelMap = [Int: NSRunningApplication]()
    getWindowsInAllSpaces().enumerated().forEach { (index: Int, cgWindowId: CGWindowID) in
        if let pid = windowPidMap[Int(cgWindowId)] {
            if let app = runningApps.first(where: { Int($0.processIdentifier) == pid }) {
                if app.activationPolicy == .regular {
                    windowLevelMap[index] = app
                }
            }
        }
    }

    // Add any applications that are running but don't have windows (not fully closed and therefore still in the dock).
    runningApps.enumerated().forEach { (index: Int, app: NSRunningApplication) in
        if app.activationPolicy == .regular && !windowLevelMap.values.contains(app) {
           let baseIndex = windowLevelMap.count
           windowLevelMap[baseIndex + index] = app
        }
    }

    var seenNames = Set<String>()
    windowLevelMap.sorted { $0.key < $1.key }.forEach { (key: Int, app: NSRunningApplication) in
        if let name = app.localizedName {
            if !seenNames.contains(name) {
                if let iconPath = saveIconToFile(icon: app.icon, name: name) {
                    result.append(Application(name: name, icon_path: iconPath, pid: Int(app.processIdentifier)))
                    seenNames.insert(name)
                }
            }
        }
    }

    return SRObjectArray(result)
}

func getWindowsInAllSpaces() -> [CGWindowID] {
    var visibleSpaces = [CGSSpaceID]()

    (CGSCopyManagedDisplaySpaces(CGS_CONNECTION) as! [NSDictionary]).forEach { (screen: NSDictionary) in
        (screen["Spaces"] as! [NSDictionary]).forEach { (space: NSDictionary) in
            let spaceId = space["id64"] as! CGSSpaceID
            visibleSpaces.append(spaceId)

        }
    }

    var set_tags = 0
    var clear_tags = 0x4000000000
    return CGSCopyWindowsWithOptionsAndTags(CGS_CONNECTION, 0, visibleSpaces as CFArray, 2, &set_tags, &clear_tags) as! [CGWindowID]
}

func saveIconToFile(icon: NSImage?, name: String) -> String? {
    guard let unwrappedIcon = icon,
        let pngData = unwrappedIcon.pngRepresentation else {
        return nil
    }

    let tempDirectory = FileManager.default.temporaryDirectory
    let iconDirectory = tempDirectory.appendingPathComponent("com.gaauwe.fast-forward")
    let iconFileName = "\(name).png"
    let iconPath = iconDirectory.appendingPathComponent(iconFileName)

    if !FileManager.default.fileExists(atPath: iconDirectory.path) {
        try? FileManager.default.createDirectory(at: iconDirectory, withIntermediateDirectories: true)
    }

    if !FileManager.default.fileExists(atPath: iconPath.path) {
        do {
            try pngData.write(to: iconPath)
        } catch {
            print("Failed to write PNG for \(name): \(error)")
        }
    }

    return iconPath.path
}

extension NSImage {
    var pngRepresentation: Data? {
        let size = NSSize(width: 64, height: 64)
        let resizedImage = NSImage(size: size)

        resizedImage.lockFocus()
        self.draw(in: NSRect(origin: .zero, size: size),
                 from: NSRect(origin: .zero, size: self.size),
                 operation: .copy,
                 fraction: 1.0)
        resizedImage.unlockFocus()

        guard let tiffData = resizedImage.tiffRepresentation,
              let bitmap = NSBitmapImageRep(data: tiffData) else {
            return nil
        }
        return bitmap.representation(using: .png, properties: [:])
    }
}
