package app.prisma.vpn

import android.app.Activity
import android.content.Intent
import android.net.VpnService
import android.webkit.WebView
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

private const val VPN_REQUEST_CODE = 24601

@InvokeArg
class StartServiceArgs {
    var handle: Long = 0
}

@TauriPlugin
class VpnPlugin(private val activity: Activity) : Plugin(activity) {

    // Pending invoke waiting for the permission activity result
    private var pendingPermissionInvoke: Invoke? = null

    // Client handle passed from Rust to the VPN service
    private var clientHandle: Long = 0

    /**
     * Check whether VPN permission has been granted.
     *
     * On Android, `VpnService.prepare(context)` returns `null` when the app
     * already has VPN permission, or an Intent to request it otherwise.
     */
    @Command
    fun checkPermission(invoke: Invoke) {
        val intent = VpnService.prepare(activity)
        val result = JSObject()
        result.put("granted", intent == null)
        invoke.resolve(result)
    }

    /**
     * Request VPN permission by launching the system consent dialog.
     *
     * If permission is already granted, resolves immediately with `granted: true`.
     * Otherwise, starts the system VPN permission Activity and resolves when
     * the user accepts or denies.
     */
    @Command
    fun requestPermission(invoke: Invoke) {
        val intent = VpnService.prepare(activity)
        if (intent == null) {
            // Already granted
            val result = JSObject()
            result.put("granted", true)
            invoke.resolve(result)
            return
        }

        // Store the invoke so we can resolve it in onActivityResult
        pendingPermissionInvoke = invoke
        activity.startActivityForResult(intent, VPN_REQUEST_CODE)
    }

    /**
     * Handle the result from the VPN permission Activity.
     */
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode == VPN_REQUEST_CODE) {
            val granted = resultCode == Activity.RESULT_OK
            val pending = pendingPermissionInvoke
            pendingPermissionInvoke = null
            if (pending != null) {
                val result = JSObject()
                result.put("granted", granted)
                pending.resolve(result)
            }
        }
    }

    /**
     * Start the PrismaVpnService with the given client handle.
     *
     * The handle is passed via the Intent extra so the service can call
     * nativeSetTunFd() to hand the TUN fd to Rust.
     */
    @Command
    fun startService(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(StartServiceArgs::class.java)
            clientHandle = args.handle

            // Verify permission first
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

    /**
     * Stop the PrismaVpnService.
     */
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
