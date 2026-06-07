#!/bin/bash

# Rigorous Testing Runner for the ZEUS Compiler
# This script iterates over all .shy files in tests/passed/
# It expects them to compile to C successfully, compile to executable, and run.

echo "======================================"
echo "    ZEUS Compiler Test Runner"
echo "======================================"
echo

COMPILER="./zeus" # Assuming the compiled zeus binary is in the root directory

if [ ! -f "$COMPILER" ]; then
    echo "Error: ZEUS compiler executable '$COMPILER' not found."
    echo "Please build the compiler first before running the test suite."
    exit 1
fi

passed_tests=0
failed_tests=0

echo "Running [ROUND-TRIP & EXECUTION TESTS] in tests/passed/"
echo "--------------------------------------------------------"

# Check if there are any .zs files in the passed directory
shopt -s nullglob
passed_files=(tests/passed/*.zs)

if [ ${#passed_files[@]} -eq 0 ]; then
    echo "No test files found in tests/passed/"
else
    for file in "${passed_files[@]}"; do
        echo -n "Testing $file... "
        
        base_name=$(basename "$file" .zs)
        $COMPILER "$file" > compile_log.txt 2> compile_err.log
        if [ $? -ne 0 ]; then
            echo "FAILED (ZEUS compilation error)"
            cat compile_err.log
            ((failed_tests++))
            continue
        fi
        
        # 2. Compile: Compile .c to executable
        gcc "${base_name}.c" -o test_bin 2> gcc_err.log
        if [ $? -ne 0 ]; then
            echo "FAILED (GCC compilation error)"
            cat gcc_err.log
            ((failed_tests++))
            continue
        fi
        
        # 3. Run: Execute the compiled program
        ./test_bin > run_output.log 2>&1
        if [ $? -ne 0 ]; then
            echo "FAILED (Runtime execution error)"
            cat run_output.log
            ((failed_tests++))
            continue
        fi
        
        # Verify: Here you can extend the script to compare run_output.log with expected outputs.
        echo "PASSED"
        ((passed_tests++))
    done
fi

echo
echo "Running [NEGATIVE TESTS] in tests/failed/"
echo "--------------------------------------------------------"
failed_files=(tests/failed/*.zs)

if [ ${#failed_files[@]} -eq 0 ]; then
    echo "No test files found in tests/failed/"
else
    for file in "${failed_files[@]}"; do
        echo -n "Testing negative case $file... "
        
        # We expect ZEUS to fail and exit with a non-zero status
        $COMPILER "$file" > /dev/null 2> compile_err.log
        if [ $? -eq 0 ]; then
            echo "FAILED (ZEUS incorrectly compiled invalid code)"
            ((failed_tests++))
        else
            echo "PASSED (ZEUS successfully caught error)"
            ((passed_tests++))
        fi
    done
fi

echo
echo "======================================"
echo "    TEST SUMMARY"
echo "======================================"
echo "Total Passed: $passed_tests"
echo "Total Failed: $failed_tests"

# Cleanup
rm -f output.c test_bin compile_err.log gcc_err.log run_output.log

if [ $failed_tests -ne 0 ]; then
    exit 1
fi

exit 0
