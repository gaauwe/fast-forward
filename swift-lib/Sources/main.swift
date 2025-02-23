import AppKit
import ApplicationServices
import Cocoa
import Foundation

// Logger class to handle file logging
class Logger {
    static let shared = Logger()
    private let fileHandle: FileHandle?
    private let dateFormatter: DateFormatter
    private let logQueue = DispatchQueue(label: "com.swift.monitor.logger")

    private init() {
        dateFormatter = DateFormatter()
        dateFormatter.dateFormat = "yyyy-MM-dd HH:mm:ss.SSS"

        // Create logs directory if it doesn't exist
        let fileManager = FileManager.default
        let logDir = "/Users/gaauwe/Library/Application Support/FastForward"
        let logPath = "\(logDir)/swift.log"

        do {
            try fileManager.createDirectory(atPath: logDir, withIntermediateDirectories: true)
            if !fileManager.fileExists(atPath: logPath) {
                fileManager.createFile(atPath: logPath, contents: nil)
            }
            fileHandle = try FileHandle(forWritingTo: URL(fileURLWithPath: logPath))
            fileHandle?.seekToEndOfFile()
        } catch {
            print("Failed to setup logging: \(error)")
            fileHandle = nil
        }
    }

    func log(_ message: String, level: String = "INFO") {
        let timestamp = dateFormatter.string(from: Date())
        let logMessage = "[\(timestamp)] [\(level)] \(message)\n"

        logQueue.async { [weak self] in
            print(logMessage, terminator: "")
            if let data = logMessage.data(using: .utf8) {
                self?.fileHandle?.write(data)
            }
        }
    }

    deinit {
        try? fileHandle?.close()
    }
}

let CGS_CONNECTION = CGSMainConnectionID()
typealias CGSConnectionID = UInt32
typealias CGSSpaceID = UInt64

@_silgen_name("CGSCopyWindowsWithOptionsAndTags")
func CGSCopyWindowsWithOptionsAndTags(
    _ cid: CGSConnectionID, _ owner: Int, _ spaces: CFArray, _ options: Int, _ setTags: inout Int,
    _ clearTags: inout Int
) -> CFArray

@_silgen_name("CGSCopyManagedDisplaySpaces")
func CGSCopyManagedDisplaySpaces(_ cid: CGSConnectionID) -> CFArray

@_silgen_name("CGSMainConnectionID")
func CGSMainConnectionID() -> CGSConnectionID

struct AppProfile: Codable {
    let _name: String
    let path: String
}

struct AppProperty: Codable {
    let _items: [AppProfile]
}

class AppLifecycleMonitor: @unchecked Sendable {
    private let socketPath = "/tmp/swift_monitor.sock"
    private var socketFileDescriptor: Int32 = -1
    private var clientSocket: Int32 = -1  // Store the client socket
    private let socketQueue = DispatchQueue(label: "com.swift.monitor.socketQueue")
    private var source: DispatchSourceRead?

    private func log(_ message: Any, level: String = "INFO") {
        Logger.shared.log("\(message)", level: level)
    }

    init() {
        startSocket()
        startListening()
    }

    private func startSocket() {
        // Remove any existing socket file
        unlink(socketPath)

        // Create a new Unix domain socket
        socketFileDescriptor = socket(AF_UNIX, SOCK_STREAM, 0)
        guard socketFileDescriptor >= 0 else {
            self.log("Failed to create socket with error: \(errno)", level: "ERROR")
            fatalError("Failed to create socket")
        }
        self.log("Socket created: \(socketFileDescriptor)")

        var addr = sockaddr_un()
        addr.sun_family = sa_family_t(AF_UNIX)
        strncpy(&addr.sun_path.0, socketPath, MemoryLayout.size(ofValue: addr.sun_path) - 1)

        let len = socklen_t(MemoryLayout.size(ofValue: addr))
        let bindResult = withUnsafePointer(to: &addr) {
            $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                bind(socketFileDescriptor, $0, len)
            }
        }

        guard bindResult == 0 else {
            self.log("Failed to bind socket with error: \(errno)", level: "ERROR")
            fatalError("Failed to bind socket")
        }
        print("Socket bound successfully")
        fflush(stdout)
        self.log("Socket bound successfully")

        guard listen(socketFileDescriptor, 10) == 0 else {
            self.log("Failed to listen on socket with error: \(errno)", level: "ERROR")
            fatalError("Failed to listen on socket")
        }
        self.log("Listening on socket \(socketPath)")

        // Set socket to non-blocking mode
        var flags = fcntl(socketFileDescriptor, F_GETFL, 0)
        flags |= O_NONBLOCK
        var _ = fcntl(socketFileDescriptor, F_SETFL, flags)

        // Use DispatchSource to handle incoming connections asynchronously
        source = DispatchSource.makeReadSource(fileDescriptor: socketFileDescriptor, queue: socketQueue)
        source?.setEventHandler { [weak self] in
            self?.acceptClientConnection()
        }
        source?.resume()
    }

    private func acceptClientConnection() {
        // Accept a new client connection if it's not already established
        if clientSocket == -1 {
            let client = accept(self.socketFileDescriptor, nil, nil)
            if client >= 0 {
                clientSocket = client
                self.log("Client connected, clientSocket: \(clientSocket)")

                // Send the initial application list when a client connects
                let running_apps = getApplicationWindows()
                let installed_apps = getInstalledApplications()
                let filtered_installed_apps = installed_apps.filter { installed_app in
                    !running_apps.contains { running_app in
                        running_app.path == installed_app.path
                    }
                }
                let apps = running_apps + filtered_installed_apps

                let message = SocketMessage(event: List(apps: apps))
                sendMessageToClient(message)
            } else {
                if errno != EAGAIN {
                    self.log("Failed to accept client with error: \(errno)", level: "ERROR")
                }
            }
        }
    }

    private func sendMessageToClient(_ message: SocketMessage) {
        do {
            let data = try message.serializedData()

            // Prefix the message with its length
            var length = UInt32(data.count).bigEndian
            let lengthData = Data(bytes: &length, count: MemoryLayout<UInt32>.size)

            // Combine length and message data
            var combinedData = lengthData
            combinedData.append(data)
            self.log("Sending message with length: \(data.count)")

            socketQueue.async { [weak self] in
                guard let self = self else { return }

                // Check if there is a valid client socket
                if self.clientSocket >= 0 {
                    var totalBytesSent = 0
                    while totalBytesSent < combinedData.count {
                        let bytesLeft = combinedData.count - totalBytesSent
                        let sentBytes = combinedData.withUnsafeBytes {
                            send(
                                self.clientSocket, $0.baseAddress!.advanced(by: totalBytesSent),
                                bytesLeft, 0)
                        }
                        if sentBytes < 0 {
                            let errorString = String(cString: strerror(errno))
                            if errno == EAGAIN || errno == EWOULDBLOCK {
                                self.log("Send buffer full, retrying...", level: "WARN")
                                usleep(100_000)  // Sleep for 100 milliseconds
                                continue
                            } else {
                                self.log(
                                    "Failed to send data, sentBytes: \(sentBytes), error: \(errorString)",
                                    level: "ERROR"
                                )
                                break
                            }
                        }
                        totalBytesSent += sentBytes
                        self.log("Sent \(sentBytes) bytes, total sent: \(totalBytesSent) bytes.")
                    }

                    if totalBytesSent == combinedData.count {
                        self.log("All bytes sent successfully.")
                    } else {
                        self.log("Warning: Not all bytes were sent.", level: "WARN")
                    }
                } else {
                    self.log("No client connected, cannot send message.", level: "WARN")
                }
            }
        } catch {
            self.log("Failed to send message: \(error)", level: "ERROR")
        }
    }

    private func startListening() {
        let workspace = NSWorkspace.shared
        let notificationCenter = workspace.notificationCenter

        // Listen for application launch events
        notificationCenter.addObserver(
            self,
            selector: #selector(applicationEvent(_:)),
            name: NSWorkspace.didLaunchApplicationNotification,
            object: nil
        )

        // Listen for application termination events
        notificationCenter.addObserver(
            self,
            selector: #selector(applicationEvent(_:)),
            name: NSWorkspace.didTerminateApplicationNotification,
            object: nil
        )

        // Listen for application activation events
        notificationCenter.addObserver(
            self,
            selector: #selector(applicationEvent(_:)),
            name: NSWorkspace.didActivateApplicationNotification,
            object: nil
        )

        self.log("Started listening for application events")

        // Start the run loop to keep the application running
        RunLoop.main.run()
    }

    func getInstalledApplications() -> [App] {
        let task = Process()
        let pipe = Pipe()

        task.launchPath = "/usr/sbin/system_profiler"
        task.arguments = ["-xml", "-detailLevel", "mini", "SPApplicationsDataType"]
        task.standardOutput = pipe
        task.launch()

        // parse the plist data
        let data = pipe.fileHandleForReading.readDataToEndOfFile()
        if let decoder = try? PropertyListDecoder().decode([AppProperty].self, from: data),
            var apps = decoder.first?._items
        {
            let new_apps =
                apps
                .filter({
                    $0.path.hasPrefix("/Applications/")
                        || $0.path.hasPrefix("/System/Applications/")
                })
                .map {
                    let icon = NSWorkspace.shared.icon(forFile: $0.path)
                    let path = saveIconToFile(icon: icon, name: $0._name) ?? ""
                    return App(name: $0._name, pid: 0, icon: path, active: false, path: $0.path)
                }
            return new_apps

        }
        return []
    }

    func getApplicationWindows() -> [App] {
        var result: [App] = []
        let runningApps = NSWorkspace.shared.runningApplications

        var windowPidMap = [Int: Int]()
        (CGWindowListCopyWindowInfo([.excludeDesktopElements, .optionAll], 0) as! [[String: Any]])
            .forEach { (item: [String: Any]) in
                if let windowNumber = item[kCGWindowNumber as String] as? Int,
                    let ownerPID = item[kCGWindowOwnerPID as String] as? Int
                {
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
                    result.append(getAppData(app))
                    seenNames.insert(name)
                }
            }
        }

        return result
    }

    func getWindowsInAllSpaces() -> [CGWindowID] {
        var visibleSpaces = [CGSSpaceID]()

        (CGSCopyManagedDisplaySpaces(CGS_CONNECTION) as! [NSDictionary]).forEach {
            (screen: NSDictionary) in
            (screen["Spaces"] as! [NSDictionary]).forEach { (space: NSDictionary) in
                let spaceId = space["id64"] as! CGSSpaceID
                visibleSpaces.append(spaceId)

            }
        }

        var set_tags = 0
        var clear_tags = 0x40_0000_0000
        return CGSCopyWindowsWithOptionsAndTags(
            CGS_CONNECTION, 0, visibleSpaces as CFArray, 2, &set_tags, &clear_tags) as! [CGWindowID]
    }

    func getAppData(_ app: NSRunningApplication) -> App {
        let name = app.localizedName ?? "Unknown"

        return App(
            name: name,
            pid: Int32(app.processIdentifier),
            icon: saveIconToFile(icon: app.icon, name: name) ?? "",
            active: app.isActive,
            path: app.bundleURL?.path ?? ""
        )
    }

    func saveIconToFile(icon: NSImage?, name: String) -> String? {
        guard let unwrappedIcon = icon,
            let pngData = unwrappedIcon.pngRepresentation
        else {
            return nil
        }

        let tempDirectory = FileManager.default.temporaryDirectory
        let iconDirectory = tempDirectory.appendingPathComponent("com.gaauwe.fast-forward")
        let iconFileName = "\(name) (icon).png"
        let iconPath = iconDirectory.appendingPathComponent(iconFileName)

        if !FileManager.default.fileExists(atPath: iconDirectory.path) {
            try? FileManager.default.createDirectory(
                at: iconDirectory, withIntermediateDirectories: true)
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

    @objc private func applicationEvent(_ notification: Notification) {
        if let userInfo = notification.userInfo,
            let app = userInfo[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication
        {
            if app.activationPolicy != .regular { return }

            let event: Sendable
            switch notification.name {
            case NSWorkspace.didLaunchApplicationNotification:
                event = Launch(app: getAppData(app))
            case NSWorkspace.didTerminateApplicationNotification:
                event = Close(app: getAppData(app))
            default:
                event = Activate(app: getAppData(app))
            }

            sendMessageToClient(SocketMessage(event: event))
        }
    }

    deinit {
        // Cleanup: Remove observers and close the socket when the app is deinitialized
        NSWorkspace.shared.notificationCenter.removeObserver(self)
        source?.cancel()  // Cancel DispatchSource
        close(socketFileDescriptor)
        if clientSocket >= 0 {
            close(clientSocket)
        }
        // Remove the socket file
        unlink(socketPath)
    }
}

extension NSImage {
    var pngRepresentation: Data? {
        let size = NSSize(width: 64, height: 64)
        let resizedImage = NSImage(size: size)

        resizedImage.lockFocus()
        self.draw(
            in: NSRect(origin: .zero, size: size),
            from: NSRect(origin: .zero, size: self.size),
            operation: .copy,
            fraction: 1.0)
        resizedImage.unlockFocus()

        guard let tiffData = resizedImage.tiffRepresentation,
            let bitmap = NSBitmapImageRep(data: tiffData)
        else {
            return nil
        }
        return bitmap.representation(using: .png, properties: [:])
    }
}

// MARK: - Protobuf Extensions
extension App {
    init(name: String, pid: Int32, icon: String, active: Bool, path: String) {
        self.name = name
        self.pid = pid
        self.icon = icon
        self.active = active
        self.path = path
    }
}

extension SocketMessage {
    init(event: Sendable) {
        switch event {
        case let event as Launch:
            self.event = .launch(event)
        case let event as Close:
            self.event = .close(event)
        case let event as Activate:
            self.event = .activate(event)
        case let event as List:
            self.event = .list(event)
        default:
            fatalError("Unknown event type")
        }
    }
}

extension Launch {
    init(app: App) {
        self.app = app
    }
}

extension Close {
    init(app: App) {
        self.app = app
    }
}

extension Activate {
    init(app: App) {
        self.app = app
    }
}

extension List {
    init(apps: [App]) {
        self.apps = apps
    }
}

// Start the app lifecycle monitor
let monitor = AppLifecycleMonitor()
print("Listening for app launch/close events...")
