#!/bin/bash
# Zeus test runner: strips ANSI from output, checks expected lines appear
BINARY="$1"
CASES="$2"

strip_ansi() { sed 's/\x1b\[[0-9;]*m//g; s/\x1b\[[0-9;]*[A-Za-z]//g'; }
strip_timing() { sed 's/[0-9][0-9]*\.[0-9]*ms/Xms/g; s/[ ][0-9]*µs/Xµs/g'; }

pass=0; fail=0; neg_pass=0; neg_fail=0

for input in "$CASES"/*.zs; do
    name="$(basename "${input%.zs}")"
    expected="$CASES/${name}.expected"

    if [[ "$name" == neg_* ]]; then
        actual=$("$BINARY" "$input" 2>&1); rc=$?
        if [ $rc -ne 0 ]; then
            neg_pass=$((neg_pass+1)); echo "PASS [neg] $name"
        else
            neg_fail=$((neg_fail+1)); echo "FAIL [neg] $name  (expected nonzero)"
            echo "  output: $(echo "$actual" | strip_ansi | tail -3)"
        fi
        continue
    fi

    [ -f "$expected" ] || continue

    actual_stripped=$(echo "$("$BINARY" "$input" 2>&1)" | strip_ansi)
    # Each line in .expected must appear verbatim in the stripped output
    all_found=1
    while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        if ! echo "$actual_stripped" | grep -qF "$line"; then
            all_found=0
            echo "FAIL $name  (missing: '$line')"
            break
        fi
    done < "$expected"

    if [ $all_found -eq 1 ]; then
        pass=$((pass+1)); echo "PASS $name"
    else
        fail=$((fail+1))
    fi
done

echo ""
echo "=== RESULTS ==="
echo "  Positive: $pass passed, $fail failed"
echo "  Negative: $neg_pass passed, $neg_fail failed"
echo "  Total: $((pass+neg_pass)) / $((pass+fail+neg_pass+neg_fail))"
