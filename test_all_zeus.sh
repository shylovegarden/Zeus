#!/bin/bash

# Zeus Test Suite - Run all programs and generate report
# Usage: bash test_all_zeus.sh

ZEUS_COMPILER="/Users/shy/Developer/ZEUS/zeus_compiler/target/release/zeus_compiler"
ZEUS_DIR="/Users/shy/Developer/ZEUS"
RESULTS_FILE="/tmp/zeus_test_results.txt"

echo "╔══════════════════════════════════════════════════════════════════════════════╗" | tee $RESULTS_FILE
echo "║                    🧪 ZEUS COMPREHENSIVE TEST SUITE 🧪                       ║" | tee -a $RESULTS_FILE
echo "╚══════════════════════════════════════════════════════════════════════════════╝" | tee -a $RESULTS_FILE
echo "" | tee -a $RESULTS_FILE

PROGRAMS=(
    "crypto_lookup"
    "token_validator"
    "test_boundary"
    "test_proof_verify"
    "test_stress"
)

PASS_COUNT=0
FAIL_COUNT=0

for prog in "${PROGRAMS[@]}"; do
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" | tee -a $RESULTS_FILE
    echo "Testing: $prog" | tee -a $RESULTS_FILE
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" | tee -a $RESULTS_FILE
    
    # Run the program
    if $ZEUS_COMPILER run $ZEUS_DIR/$prog.zs > /tmp/$prog.output.txt 2>&1; then
        echo "✅ BUILD: SUCCESS" | tee -a $RESULTS_FILE
        echo "✅ EXECUTION: SUCCESS" | tee -a $RESULTS_FILE
        
        # Extract output
        OUTPUT=$(tail -5 /tmp/$prog.output.txt | head -1)
        echo "   Output: $OUTPUT" | tee -a $RESULTS_FILE
        
        # Check certificate exists
        if [ -f "$ZEUS_DIR/zeus_compiler/$prog.zcert" ]; then
            echo "✅ CERTIFICATE: GENERATED" | tee -a $RESULTS_FILE
            
            # Extract constant-time status
            CT_STATUS=$(jq '.functions[0].constant_time' $ZEUS_DIR/zeus_compiler/$prog.zcert)
            echo "   Constant-Time: $CT_STATUS" | tee -a $RESULTS_FILE
            
            # Extract WCET
            WCET=$(jq '.functions[0].wcet_steps' $ZEUS_DIR/zeus_compiler/$prog.zcert)
            echo "   WCET: $WCET steps" | tee -a $RESULTS_FILE
            
            echo "✅ TEST PASSED" | tee -a $RESULTS_FILE
            ((PASS_COUNT++))
        else
            echo "❌ CERTIFICATE: NOT FOUND" | tee -a $RESULTS_FILE
            echo "❌ TEST FAILED" | tee -a $RESULTS_FILE
            ((FAIL_COUNT++))
        fi
    else
        echo "❌ BUILD: FAILED" | tee -a $RESULTS_FILE
        cat /tmp/$prog.output.txt | tee -a $RESULTS_FILE
        echo "❌ TEST FAILED" | tee -a $RESULTS_FILE
        ((FAIL_COUNT++))
    fi
    echo "" | tee -a $RESULTS_FILE
done

echo "╔══════════════════════════════════════════════════════════════════════════════╗" | tee -a $RESULTS_FILE
echo "║                            TEST SUMMARY                                      ║" | tee -a $RESULTS_FILE
echo "╚══════════════════════════════════════════════════════════════════════════════╝" | tee -a $RESULTS_FILE
echo "✅ PASSED: $PASS_COUNT" | tee -a $RESULTS_FILE
echo "❌ FAILED: $FAIL_COUNT" | tee -a $RESULTS_FILE
echo "TOTAL: $((PASS_COUNT + FAIL_COUNT))" | tee -a $RESULTS_FILE
echo "" | tee -a $RESULTS_FILE

if [ $FAIL_COUNT -eq 0 ]; then
    echo "✅ ALL TESTS PASSED!" | tee -a $RESULTS_FILE
    exit 0
else
    echo "❌ SOME TESTS FAILED" | tee -a $RESULTS_FILE
    exit 1
fi
