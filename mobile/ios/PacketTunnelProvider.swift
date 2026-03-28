import NetworkExtension
import os.log

/// iOS Network Extension packet tunnel provider.
///
/// Flow:
/// 1. User toggles VPN in the app or Settings
/// 2. iOS calls startTunnel() in this extension process
/// 3. We configure network settings and extract the TUN fd
/// 4. fd is passed to Rust via C FFI (prisma_ios_set_tun_fd)
/// 5. Rust TUN handler reads/writes packets through the fd
/// 6. Extension stays alive as long as VPN is connected
///
/// Note: This runs in a SEPARATE PROCESS from the main app.
/// Use App Groups for shared data.
class PacketTunnelProvider: NEPacketTunnelProvider {

    private let logger = Logger(subsystem: "com.prisma.client.tunnel", category: "tunnel")

    override func startTunnel(options: [String: NSObject]? = nil) async throws {
        logger.info("Starting Prisma tunnel...")

        // Configure tunnel network settings
        let settings = NEPacketTunnelNetworkSettings(tunnelRemoteAddress: "10.0.85.254")

        // IPv4 settings
        let ipv4 = NEIPv4Settings(addresses: ["10.0.85.1"], subnetMasks: ["255.255.255.0"])
        ipv4.includedRoutes = [NEIPv4Route.default()]
        settings.ipv4Settings = ipv4

        // DNS settings
        settings.dnsSettings = NEDNSSettings(servers: ["8.8.8.8", "8.8.4.4"])

        // MTU
        settings.mtu = NSNumber(value: 1500)

        // Apply network settings
        try await setTunnelNetworkSettings(settings)
        logger.info("Tunnel network settings applied")

        // Extract the TUN file descriptor from NEPacketTunnelProvider
        // This is a private API but widely used and stable
        guard let fd = self.value(forKey: "socket.fileDescriptor") as? Int32, fd >= 0 else {
            // Fallback: use packet flow API for reading/writing
            logger.warning("Could not extract TUN fd — using packet flow API")
            startPacketFlowRelay()
            return
        }

        // Pass fd to Rust via C FFI
        prisma_ios_set_tun_fd(nil, fd)
        logger.info("TUN fd \(fd) passed to Rust")
    }

    override func stopTunnel(with reason: NEProviderStopReason) async {
        logger.info("Stopping tunnel, reason: \(String(describing: reason))")
        // Reset the fd so Rust stops reading
        prisma_ios_set_tun_fd(nil, -1)
    }

    override func handleAppMessage(_ messageData: Data) async -> Data? {
        // Handle IPC messages from the main app
        // e.g., config updates, stats requests
        return nil
    }

    /// Fallback: relay packets via NEPacketTunnelProvider's packet flow API.
    /// Used when the private fd extraction doesn't work.
    private func startPacketFlowRelay() {
        // Read packets from the TUN interface
        readPackets()
    }

    private func readPackets() {
        packetFlow.readPackets { [weak self] packets, protocols in
            guard let self = self else { return }
            for (i, packet) in packets.enumerated() {
                // Forward each packet to Rust for processing
                // For now, log and continue
                _ = packet
                _ = protocols[i]
            }
            // Continue reading
            self.readPackets()
        }
    }
}

// C FFI function declarations (from prisma-ffi/include/prisma_ffi.h)
// These are linked from the prisma-ffi static library
@_silgen_name("prisma_ios_set_tun_fd")
func prisma_ios_set_tun_fd(_ handle: UnsafeMutableRawPointer?, _ fd: Int32) -> Int32

@_silgen_name("prisma_ios_prepare_tunnel_config")
func prisma_ios_prepare_tunnel_config(_ json: UnsafePointer<CChar>?) -> Int32
