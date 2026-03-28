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
 * Android VPN service that creates a TUN interface and passes the fd to Rust.
 *
 * Package: com.prisma.client — must match JNI exports in prisma-ffi/src/android.rs
 * (Java_com_prisma_client_PrismaVpnService_nativeSetTunFd)
 */
class PrismaVpnService : VpnService() {

    companion object {
        private const val TAG = "PrismaVPN"
        private const val CHANNEL_ID = "prisma_vpn"
        private const val NOTIFICATION_ID = 1
        private const val TUN_MTU = 1500

        private var vpnInterface: ParcelFileDescriptor? = null

        fun isRunning(): Boolean = vpnInterface != null

        init {
            // Load the native library containing JNI exports (prisma-ffi)
            System.loadLibrary("prisma_ffi")
        }
    }

    private var coreHandle: Long = 0

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == "STOP") {
            stopVpn()
            return START_NOT_STICKY
        }

        coreHandle = intent?.getLongExtra("CORE_HANDLE", 0) ?: 0

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
            .addDnsServer("8.8.8.8")
            .addDnsServer("8.8.4.4")

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.LOLLIPOP) {
            try {
                builder.addDisallowedApplication(packageName)
            } catch (_: Exception) { }
        }

        val pfd = builder.establish()
            ?: throw IllegalStateException("VPN permission not granted — call VpnService.prepare() first")

        vpnInterface = pfd
        val fd = pfd.fd
        Log.i(TAG, "TUN interface established, fd=$fd")

        if (coreHandle != 0L) {
            nativeSetTunFd(coreHandle, fd)
            Log.i(TAG, "TUN fd passed to Rust (handle=$coreHandle)")
        } else {
            Log.w(TAG, "Core handle not set — cannot pass TUN fd to Rust")
        }

        startForeground(NOTIFICATION_ID, buildNotification())
    }

    private fun stopVpn() {
        vpnInterface?.close()
        vpnInterface = null

        if (coreHandle != 0L) {
            nativeSetTunFd(coreHandle, -1)
        }

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

    // JNI native method — implemented in prisma-ffi/src/android.rs
    // Name matches: Java_com_prisma_client_PrismaVpnService_nativeSetTunFd
    private external fun nativeSetTunFd(handle: Long, fd: Int): Int
}
