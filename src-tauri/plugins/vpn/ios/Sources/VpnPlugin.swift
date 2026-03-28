import NetworkExtension
import Tauri
import UIKit

class VpnPlugin: Plugin {

    private var vpnManager: NETunnelProviderManager?

    /// Check whether VPN permission is granted by looking for an existing
    /// VPN configuration profile created by this app.
    @objc func checkPermission(_ invoke: Invoke) {
        NETunnelProviderManager.loadAllFromPreferences { managers, error in
            if let error = error {
                invoke.reject("Failed to check VPN permission: \(error.localizedDescription)")
                return
            }
            // If a manager exists for our app, permission was granted
            let granted = managers?.isEmpty == false
            invoke.resolve(["granted": granted])
        }
    }

    /// Request VPN permission by saving a VPN configuration profile.
    /// iOS shows a system alert the first time a VPN config is saved:
    /// "App would like to add VPN Configurations. Allow / Don't Allow"
    @objc func requestPermission(_ invoke: Invoke) {
        loadOrCreateManager { [weak self] manager, error in
            guard let manager = manager else {
                invoke.reject("Failed to load VPN configuration: \(error?.localizedDescription ?? "unknown")")
                return
            }
            self?.vpnManager = manager

            // Saving the manager triggers the iOS VPN permission dialog
            manager.saveToPreferences { error in
                if let error = error {
                    invoke.resolve(["granted": false])
                    return
                }
                // After save, reload to get fresh state
                manager.loadFromPreferences { error in
                    invoke.resolve(["granted": error == nil])
                }
            }
        }
    }

    /// Start the VPN tunnel via NETunnelProviderManager.
    @objc func startService(_ invoke: Invoke) {
        guard let args = invoke.parseArgs(StartServiceArgs.self) else {
            invoke.reject("Invalid arguments")
            return
        }

        loadOrCreateManager { [weak self] manager, error in
            guard let manager = manager else {
                invoke.reject("VPN configuration not available: \(error?.localizedDescription ?? "unknown")")
                return
            }
            self?.vpnManager = manager

            // Save + start in sequence
            manager.saveToPreferences { saveError in
                if let saveError = saveError {
                    invoke.reject("Failed to save VPN config: \(saveError.localizedDescription)")
                    return
                }
                manager.loadFromPreferences { loadError in
                    if let loadError = loadError {
                        invoke.reject("Failed to load VPN config: \(loadError.localizedDescription)")
                        return
                    }

                    do {
                        // Pass the client handle to the tunnel provider
                        let options: [String: NSObject] = [
                            "handle": NSNumber(value: args.handle)
                        ]
                        try manager.connection.startVPNTunnel(options: options)
                        invoke.resolve(["success": true])
                    } catch {
                        invoke.reject("Failed to start VPN: \(error.localizedDescription)")
                    }
                }
            }
        }
    }

    /// Stop the VPN tunnel.
    @objc func stopService(_ invoke: Invoke) {
        if let manager = vpnManager {
            manager.connection.stopVPNTunnel()
            invoke.resolve(["success": true])
        } else {
            // Try to find and stop any active tunnel
            NETunnelProviderManager.loadAllFromPreferences { managers, _ in
                if let manager = managers?.first {
                    manager.connection.stopVPNTunnel()
                }
                invoke.resolve(["success": true])
            }
        }
    }

    // MARK: - Helpers

    private func loadOrCreateManager(
        completion: @escaping (NETunnelProviderManager?, Error?) -> Void
    ) {
        NETunnelProviderManager.loadAllFromPreferences { managers, error in
            if let error = error {
                completion(nil, error)
                return
            }

            let manager = managers?.first ?? NETunnelProviderManager()

            // Configure the tunnel provider protocol
            let proto = NETunnelProviderProtocol()
            proto.providerBundleIdentifier = "com.prisma.client.tunnel"
            proto.serverAddress = "Prisma"
            proto.disconnectOnSleep = false

            manager.protocolConfiguration = proto
            manager.localizedDescription = "Prisma VPN"
            manager.isEnabled = true

            completion(manager, nil)
        }
    }
}

// MARK: - Argument types

@objc class StartServiceArgs: NSObject, Decodable {
    var handle: Int64 = 0
}

// MARK: - Plugin registration

@_cdecl("init_plugin_vpn")
func initPlugin() -> Plugin {
    return VpnPlugin()
}
