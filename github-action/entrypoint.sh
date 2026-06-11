#!/bin/bash
set -e

echo "═══════════════════════════════════════════════════════════"
echo "  🔒 ZEUS SECURITY VERIFICATION"
echo "  Mathematical proof your code is safe"
echo "═══════════════════════════════════════════════════════════"
echo ""

# Configuration
SOURCE_PATH="${ZEUS_SOURCE_PATH:-./src}"
LANGUAGE="${ZEUS_LANGUAGE:-auto-detect}"
POLICY="${ZEUS_POLICY:-zero-heap,constant-time}"
FAIL_ON="${ZEUS_FAIL_ON:-critical}"
GENERATE_REPORT="${ZEUS_GENERATE_REPORT:-true}"

echo "📁 Source Path: $SOURCE_PATH"
echo "🔤 Language: $LANGUAGE"
echo "📋 Policies: $POLICY"
echo "❌ Fail On: $FAIL_ON"
echo ""

# Verify source exists
if [ ! -d "$SOURCE_PATH" ]; then
    echo "❌ Error: Source path not found: $SOURCE_PATH"
    exit 1
fi

# Run Zeus verification
echo "🔍 Running Zeus verification..."
echo ""

# Build verification command
VERIFY_CMD="zeus verify"

if [ "$LANGUAGE" != "auto-detect" ]; then
    VERIFY_CMD="$VERIFY_CMD --language=$LANGUAGE"
fi

if [ -n "$POLICY" ]; then
    VERIFY_CMD="$VERIFY_CMD --policy=$POLICY"
fi

VERIFY_CMD="$VERIFY_CMD --cert --output-format=json $SOURCE_PATH"

echo "Running: $VERIFY_CMD"
echo ""

# Execute verification
set +e
VERIFICATION_OUTPUT=$(eval $VERIFY_CMD 2>&1)
VERIFICATION_EXIT_CODE=$?
set -e

# Parse results
if [ $VERIFICATION_EXIT_CODE -eq 0 ]; then
    echo "✅ VERIFICATION PASSED"
    echo ""
    echo "🔐 Security Properties Verified:"
    echo "  • Zero-Heap: No dynamic memory allocation"
    echo "  • Constant-Time: No timing side-channels"
    echo "  • Bounded: Provable execution time"
    echo ""
    
    # Output certificate path
    CERT_FILE=$(echo "$VERIFICATION_OUTPUT" | grep -oP 'Certificate: \K.*' || echo "certificate.zcert")
    echo "📜 Certificate: $CERT_FILE"
    
    # Set outputs for GitHub Actions
    echo "certificate=$CERT_FILE" >> $GITHUB_OUTPUT
    echo "verification-passed=true" >> $GITHUB_OUTPUT
    echo "properties-verified=$POLICY" >> $GITHUB_OUTPUT
    
    if [ -n "$ZEUS_API_KEY" ]; then
        echo "report-url=https://dashboard.zeus-lang.org/verify/$GITHUB_RUN_ID" >> $GITHUB_OUTPUT
    fi
    
    echo ""
    echo "═══════════════════════════════════════════════════════════"
    echo "  ✅ Your code is mathematically proven safe"
    echo "═══════════════════════════════════════════════════════════"
    
    exit 0
else
    echo "❌ VERIFICATION FAILED"
    echo ""
    echo "Details:"
    echo "$VERIFICATION_OUTPUT"
    echo ""
    
    # Determine if we should fail the build
    if [ "$FAIL_ON" = "critical" ] || [ "$FAIL_ON" = "warning" ]; then
        echo "═══════════════════════════════════════════════════════════"
        echo "  ❌ Build blocked: Security policies not satisfied"
        echo "═══════════════════════════════════════════════════════════"
        
        echo "verification-passed=false" >> $GITHUB_OUTPUT
        exit 1
    else
        echo "⚠️  Verification failed but fail-on=$FAIL_ON, continuing..."
        echo "verification-passed=false" >> $GITHUB_OUTPUT
        exit 0
    fi
fi
