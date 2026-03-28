package com.prisma.client

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Intent
import android.net.VpnService
import android.os.Build
import android.os.ParcelFileDescriptor
import android.util.Log

/**
 * Android VPN service that creates a TUN interface.
 *
 * The TUN fd is stored in a companion object field and read by the
 * Tauri VPN plugin's Rust side (which calls prisma_set_tun_fd).
 * No direct JNI calls from Kotlin — all FFI goes through the plugin.
 */
class PrismaVpnService : VpnService() {

    companion object {
        private const val TAG = "PrismaVPN"
        private const val CHANNEL_ID = "prisma_vpn"
        private const val NOTIFICATION_ID = 1
        private const val TUN_MTU = 1500

        private var vpnInterface: ParcelFileDescriptor? = null

        /** The TUN fd, readable from the plugin after service starts. -1 if not started. */
        @Volatile
        @JvmStatic
        var tunFd: Int = -1
            private set

        fun isRunning(): Boolean = vpnInterface != null
    }

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == "STOP") {
            stopVpn()
            return START_NOT_STICKY
        }

        return try {
            startVpn()
            START_STICKY
        } catch (e: Exception) {
            Log.e(TAG, "Failed to start VPN", e)
            stopSelf()
            START_NOT_STICKY
        }
    }

    private fun startVpn() {
        val builder = Builder()
            .setSession("Prisma")
            .setMtu(TUN_MTU)
            .addAddress("10.0.85.1", 24)
            .addRoute("0.0.0.0", 0)
            .addDnsServer("1.1.1.1")
            .addDnsServer("1.0.0.1")
            .addDnsServer("8.8.8.8")
            .addDnsServer("8.8.4.4")

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.LOLLIPOP) {
            try {
                builder.addDisallowedApplication(packageName)
            } catch (_: Exception) { }
        }

        val pfd = builder.establish()
            ?: throw IllegalStateException("VPN permission not granted")

        vpnInterface = pfd
        tunFd = pfd.fd
        Log.i(TAG, "TUN interface established, fd=$tunFd")

        startForeground(NOTIFICATION_ID, buildNotification())
    }

    private fun stopVpn() {
        vpnInterface?.close()
        vpnInterface = null
        tunFd = -1

        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
        Log.i(TAG, "VPN stopped")
    }

    override fun onDestroy() {
        stopVpn()
        super.onDestroy()
    }

    override fun onRevoke() {
        stopVpn()
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Prisma VPN",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Shows when Prisma VPN is active"
                setShowBadge(false)
            }
            val manager = getSystemService(NotificationManager::class.java)
            manager.createNotificationChannel(channel)
        }
    }

    private fun buildNotification(): Notification {
        val stopIntent = Intent(this, PrismaVpnService::class.java).apply {
            action = "STOP"
        }
        val stopPending = PendingIntent.getService(
            this, 0, stopIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        return Notification.Builder(this, CHANNEL_ID)
            .setContentTitle("Prisma VPN")
            .setContentText("Connected — traffic is encrypted")
            .setSmallIcon(android.R.drawable.ic_lock_lock)
            .setOngoing(true)
            .addAction(
                Notification.Action.Builder(
                    null, "Disconnect", stopPending
                ).build()
            )
            .build()
    }
}
