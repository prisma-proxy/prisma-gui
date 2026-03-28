#!/usr/bin/env bash
# android-vpn-test.sh — Automated VPN connectivity verification via ADB
# Usage: bash scripts/android-vpn-test.sh [package_name]
set -euo pipefail

PACKAGE="${1:-com.prisma.client}"
TIMEOUT_TUN=15      # seconds to wait for TUN interface
TIMEOUT_FD=30       # seconds to wait for fd handoff
TIMEOUT_CONNECT=45  # seconds to wait for full connection
LOGCAT_FILE=$(mktemp)

PASS=0; FAIL=0; WARN=0

pass()  { ((PASS++)); echo "  ✓ $1"; }
fail()  { ((FAIL++)); echo "  ✗ $1"; }
warn()  { ((WARN++)); echo "  ⚠ $1"; }

cleanup() {
    # Kill background logcat
    [[ -n "${LOGCAT_PID:-}" ]] && kill "$LOGCAT_PID" 2>/dev/null || true
    rm -f "$LOGCAT_FILE"
}
trap cleanup EXIT

# ── Pre-flight ──────────────────────────────────────────────────────────────

echo "=== Pre-flight Checks ==="

if ! adb devices 2>/dev/null | grep -q "device$"; then
    fail "No ADB device connected"
    echo "Result: $FAIL failed"; exit 1
fi
pass "ADB device connected"

if adb shell pm list packages 2>/dev/null | grep -q "$PACKAGE"; then
    pass "Package $PACKAGE installed"
else
    fail "Package $PACKAGE not installed"
    echo "Result: $FAIL failed"; exit 1
fi

# Check VPN permission
VPN_PERM=$(adb shell cmd appops get "$PACKAGE" ACTIVATE_VPN 2>/dev/null || echo "unknown")
if echo "$VPN_PERM" | grep -qi "allow"; then
    pass "VPN permission granted"
else
    warn "VPN permission status: $VPN_PERM"
fi

# ── Start Logcat Capture ────────────────────────────────────────────────────

echo ""
echo "=== Starting Logcat Monitor ==="
adb logcat -c 2>/dev/null  # Clear old logs
adb logcat -s RustStdoutStderr:I PrismaVPN:I > "$LOGCAT_FILE" 2>/dev/null &
LOGCAT_PID=$!
pass "Logcat capture started (PID $LOGCAT_PID)"

echo ""
echo "=== Waiting for VPN Events ==="
echo "  (Connect via the app UI now, or trigger programmatically)"

# ── Wait for VPN Lifecycle Events ───────────────────────────────────────────

wait_for_pattern() {
    local pattern="$1" timeout="$2" label="$3"
    local elapsed=0
    while (( elapsed < timeout )); do
        if grep -q "$pattern" "$LOGCAT_FILE" 2>/dev/null; then
            pass "$label (${elapsed}s)"
            return 0
        fi
        sleep 1
        ((elapsed++))
    done
    fail "$label (timed out after ${timeout}s)"
    return 1
}

wait_for_pattern "TUN interface established" "$TIMEOUT_TUN" "TUN interface created" || true
wait_for_pattern "Got TUN fd from VPN service" "$TIMEOUT_FD" "TUN fd received by Rust" || true
wait_for_pattern "Starting TUN mode" "$TIMEOUT_CONNECT" "TUN mode started" || true

# ── Post-Connect Verification ───────────────────────────────────────────────

echo ""
echo "=== Post-Connect Checks ==="

# Check TUN interface exists
if adb shell ip link show 2>/dev/null | grep -qi "tun"; then
    pass "TUN interface exists"
else
    fail "TUN interface not found"
fi

# Check routes
if adb shell ip route show table all 2>/dev/null | grep -qi "tun\|vpn"; then
    pass "VPN routes installed"
else
    warn "No VPN-specific routes found (may use default route)"
fi

# Check DNS resolution
if adb shell nslookup google.com 2>/dev/null | grep -q "Address"; then
    pass "DNS resolution works"
else
    fail "DNS resolution failed"
fi

# Check external IP (the real connectivity test)
echo "  Checking external IP (may take up to 10s)..."
EXT_IP=$(adb shell curl -s --max-time 10 https://api.ipify.org 2>/dev/null || echo "FAILED")
if [[ "$EXT_IP" == "FAILED" ]] || [[ -z "$EXT_IP" ]]; then
    fail "External IP check failed — VPN proxy not working"
else
    pass "External IP: $EXT_IP"
fi

# Check for port conflicts
PORTS=$(adb shell ss -tlnp 2>/dev/null | grep -cE ":1080|:8080" || echo "0")
if (( PORTS <= 2 )); then
    pass "Port bindings OK ($PORTS listeners on 1080/8080)"
else
    warn "Multiple listeners on proxy ports ($PORTS found)"
fi

# Check for errors in logs
ERROR_COUNT=$(grep -ci "error\|panic\|FATAL" "$LOGCAT_FILE" 2>/dev/null || echo "0")
if (( ERROR_COUNT == 0 )); then
    pass "No errors in logcat"
else
    warn "$ERROR_COUNT error(s) found in logcat"
    grep -i "error\|panic\|FATAL" "$LOGCAT_FILE" | tail -5
fi

# ── Summary ─────────────────────────────────────────────────────────────────

echo ""
echo "=== Summary ==="
echo "  Passed: $PASS"
echo "  Failed: $FAIL"
echo "  Warned: $WARN"

if (( FAIL > 0 )); then
    echo ""
    echo "Run 'bash scripts/android-vpn-diagnose.sh' for detailed diagnostics."
    exit 1
fi
exit 0
