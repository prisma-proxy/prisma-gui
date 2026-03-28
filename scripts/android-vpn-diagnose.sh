#!/usr/bin/env bash
# android-vpn-diagnose.sh — Dump all VPN-relevant state for debugging
# Usage: bash scripts/android-vpn-diagnose.sh [package_name]
set -euo pipefail

PACKAGE="${1:-com.prisma.client}"

if ! adb devices 2>/dev/null | grep -q "device$"; then
    echo "ERROR: No ADB device connected"
    exit 1
fi

echo "═══════════════════════════════════════════════════════════"
echo "  Prisma VPN Diagnostic Dump — $(date -Iseconds)"
echo "═══════════════════════════════════════════════════════════"

echo ""
echo "=== Device Info ==="
adb shell getprop ro.product.model 2>/dev/null || echo "(unknown)"
adb shell getprop ro.build.version.release 2>/dev/null || echo "(unknown)"

echo ""
echo "=== VPN Permission ==="
adb shell cmd appops get "$PACKAGE" ACTIVATE_VPN 2>/dev/null || echo "(unknown)"

echo ""
echo "=== Network Interfaces ==="
adb shell ip addr show 2>/dev/null | grep -E "^\d+:|inet " || echo "(none)"

echo ""
echo "=== TUN Interface ==="
adb shell ip link show 2>/dev/null | grep -A2 -i "tun" || echo "(no TUN interface)"

echo ""
echo "=== Routing Tables ==="
adb shell ip route show table all 2>/dev/null | head -30 || echo "(empty)"

echo ""
echo "=== Port Bindings (1080/8080/53) ==="
adb shell ss -tlnp 2>/dev/null | grep -E "LISTEN|:1080|:8080|:53\b" | head -20 || echo "(none)"

echo ""
echo "=== VPN Service State ==="
adb shell dumpsys activity service "$PACKAGE/.PrismaVpnService" 2>/dev/null | head -20 || echo "(not running)"

echo ""
echo "=== Connectivity Summary ==="
adb shell dumpsys connectivity 2>/dev/null | grep -A 5 "VPN" | head -15 || echo "(no VPN)"

echo ""
echo "=== DNS Resolution Test ==="
adb shell nslookup google.com 2>/dev/null | head -6 || echo "(failed)"

echo ""
echo "=== External IP Test ==="
adb shell curl -s --max-time 5 https://api.ipify.org 2>/dev/null || echo "(failed or timed out)"

echo ""
echo "=== Recent Rust Logs (last 80 lines) ==="
adb logcat -d -s RustStdoutStderr 2>/dev/null | tail -80

echo ""
echo "=== Recent VPN Logs (last 30 lines) ==="
adb logcat -d -s PrismaVPN 2>/dev/null | tail -30

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "  Dump complete"
echo "═══════════════════════════════════════════════════════════"
