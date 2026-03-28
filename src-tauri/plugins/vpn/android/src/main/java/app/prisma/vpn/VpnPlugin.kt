package app.prisma.vpn

import android.app.Activity
import android.content.Intent
import android.net.VpnService
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import com.prisma.client.PrismaVpnService

@InvokeArg
class StartServiceArgs {
    var handle: Long = 0
}

@TauriPlugin
class VpnPlugin(private val activity: Activity) : Plugin(activity) {

    @Command
    fun checkPermission(invoke: Invoke) {
        val intent = VpnService.prepare(activity)
        val result = JSObject()
        result.put("granted", intent == null)
        invoke.resolve(result)
    }

    @Command
    fun requestPermission(invoke: Invoke) {
        val intent = VpnService.prepare(activity)
        if (intent == null) {
            val result = JSObject()
            result.put("granted", true)
            invoke.resolve(result)
            return
        }
        startActivityForResult(invoke, intent, "vpnPermissionResult")
    }

    @ActivityCallback
    private fun vpnPermissionResult(invoke: Invoke, result: androidx.activity.result.ActivityResult) {
        val granted = result.resultCode == Activity.RESULT_OK
        val obj = JSObject()
        obj.put("granted", granted)
        invoke.resolve(obj)
    }

    @Command
    fun startService(invoke: Invoke) {
        try {
            val prepareIntent = VpnService.prepare(activity)
            if (prepareIntent != null) {
                invoke.reject("VPN permission not granted", "PERMISSION_DENIED")
                return
            }

            val serviceIntent = Intent(activity, PrismaVpnService::class.java).apply {
                action = "START"
            }
            activity.startForegroundService(serviceIntent)

            // Return immediately — don't poll on the main thread.
            // The Rust side will poll for the fd via MOBILE_TUN_FD atomic.
            // The VPN service sets PrismaVpnService.tunFd when TUN is created,
            // but we can't block here waiting for it (main thread deadlock).
            val result = JSObject()
            result.put("success", true)
            result.put("fd", -1) // fd not available yet, Rust polls for it
            invoke.resolve(result)
        } catch (e: Exception) {
            invoke.reject(e.message ?: "Failed to start VPN service", "START_ERROR")
        }
    }

    @Command
    fun getTunFd(invoke: Invoke) {
        val result = JSObject()
        result.put("fd", PrismaVpnService.tunFd)
        invoke.resolve(result)
    }

    @Command
    fun stopService(invoke: Invoke) {
        try {
            val serviceIntent = Intent(activity, PrismaVpnService::class.java).apply {
                action = "STOP"
            }
            activity.startService(serviceIntent)

            val result = JSObject()
            result.put("success", true)
            invoke.resolve(result)
        } catch (e: Exception) {
            invoke.reject(e.message ?: "Failed to stop VPN service", "STOP_ERROR")
        }
    }
}
