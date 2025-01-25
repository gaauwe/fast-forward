import AppKit
import ApplicationServices
import Cocoa
import Foundation

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

class AppLifecycleMonitor: @unchecked Sendable {
    private let socketPath = "/tmp/swift_monitor.sock"
    private var socketFileDescriptor: Int32 = -1
    private var clientSocket: Int32 = -1  // Store the client socket
    private let socketQueue = DispatchQueue(label: "com.swift.monitor.socketQueue")
    private var source: DispatchSourceRead?

    private func log(_ message: Any) {
        print(message)
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
            self.log("Failed to create socket with error: \(errno)")
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
            self.log("Failed to bind socket with error: \(errno)")
            fatalError("Failed to bind socket")
        }
        print("Socket bound successfully")
        fflush(stdout)

        guard listen(socketFileDescriptor, 10) == 0 else {
            self.log("Failed to listen on socket with error: \(errno)")
            fatalError("Failed to listen on socket")
        }
        self.log("Listening on socket \(socketPath)")

        // Set socket to non-blocking mode
        var flags = fcntl(socketFileDescriptor, F_GETFL, 0)
        flags |= O_NONBLOCK
        var _ = fcntl(socketFileDescriptor, F_SETFL, flags)

        // Use DispatchSource to handle incoming connections asynchronously
        source = DispatchSource.makeReadSource(
            fileDescriptor: socketFileDescriptor, queue: socketQueue)
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
                let apps = getApplicationWindows()
                let message = SocketMessage(event: List(apps: apps))
                sendMessageToClient(message)
            } else {
                if errno != EAGAIN {
                    self.log("Failed to accept client with error: \(errno)")
                }
            }
        }
    }

    private func sendMessageToClient(_ message: SocketMessage) {
        do {
            self.log("Sending message: \(message)")
            let data = try message.serializedData()

            socketQueue.async { [weak self] in
                guard let self = self else { return }

                // Check if there is a valid client socket
                if self.clientSocket >= 0 {
                    // Send data to the client
                    let sentBytes = data.withUnsafeBytes {
                        send(self.clientSocket, $0.baseAddress, $0.count, 0)
                    }
                    self.log("Sent \(sentBytes) bytes.")
                } else {
                    self.log("No client connected, cannot send message.")
                }
            }
        } catch {
            self.log("Failed to send message: \(error)")
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

        // Start the run loop to keep the application running
        RunLoop.main.run()
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
            active: app.isActive
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
    init(name: String, pid: Int32, icon: String, active: Bool) {
        self.name = name
        self.pid = pid
        self.icon = icon
        self.active = active
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

// Keep the command-line tool running
RunLoop.main.run()
