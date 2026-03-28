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

    private var clientHandle: Long = 0

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
        // Use Tauri's Plugin.startActivityForResult with a named callback
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
            val args = invoke.parseArgs(StartServiceArgs::class.java)
            clientHandle = args.handle

            val prepareIntent = VpnService.prepare(activity)
            if (prepareIntent != null) {
                invoke.reject("VPN permission not granted", "PERMISSION_DENIED")
                return
            }

            val serviceIntent = Intent(activity, PrismaVpnService::class.java).apply {
                action = "START"
                putExtra("CORE_HANDLE", clientHandle)
            }
            activity.startForegroundService(serviceIntent)

            val result = JSObject()
            result.put("success", true)
            invoke.resolve(result)
        } catch (e: Exception) {
            invoke.reject(e.message ?: "Failed to start VPN service", "START_ERROR")
        }
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
