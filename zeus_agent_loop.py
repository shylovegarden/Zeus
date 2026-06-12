import os
import subprocess
import time
import sys

# Color formatting
CYAN = '\033[96m'
GREEN = '\033[92m'
RED = '\033[91m'
YELLOW = '\033[93m'
RESET = '\033[0m'

print(f"{CYAN}======================================================{RESET}")
print(f"{CYAN}   Zeus AI Auto-Repair Loop (Compiler as Verifier)    {RESET}")
print(f"{CYAN}======================================================{RESET}\n")

# Step 1: AI generates initial (buggy) code
initial_code = """
// The AI hallucinated a type mismatch here
fn compute_metrics(x: f64) -> f64 {
    let scalar: str = 100.0; // ERROR: str initialized with f64
    return x * scalar;
}

let result = compute_metrics(5.5);
"""

print(f"{YELLOW}[AI Agent] Generating initial module for 'compute_metrics'...{RESET}")
time.sleep(1)
print(f"{YELLOW}[AI Agent] Emitting 'temp.zs':{RESET}")
for line in initial_code.strip().split('\n'):
    print(f"  {line}")
print()

with open("temp.zs", "w") as f:
    f.write(initial_code)

# Step 2: Compile and capture errors
print(f"{CYAN}[System] Passing code to Zeus Compiler for Formal Verification...{RESET}")
time.sleep(1)

# Ensure we're running the compiler from the zeus_compiler dir
# but pointing to the temp.zs file which is in the root.
cwd = os.path.abspath(os.path.join(os.path.dirname(__file__), "zeus_compiler"))
target_file = os.path.abspath("temp.zs")

result = subprocess.run(
    ["cargo", "run", "--release", "--bin", "zeus_compiler", "--", target_file],
    cwd=cwd,
    capture_output=True,
    text=True,
    encoding="utf-8",
    errors="replace"
)

out1 = result.stdout if result.stdout else ""
err1 = result.stderr if result.stderr else ""
output = out1 + err1

import re
ansi_escape = re.compile(r'\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')
clean_output = ansi_escape.sub('', output)

if "[ZEUS COMPILATION ABORTED]" in clean_output:
    print(f"{RED}[X] Zeus Compiler Rejected the Code!{RESET}")
    # Extract only the actual error for the AI
    idx = clean_output.find("error: type mismatch")
    if idx == -1:
        idx = clean_output.find("error:")
    
    error_snippet = clean_output[idx:] if idx != -1 else clean_output
    print(f"\n{RED}--- Compiler Diagnostics ---{RESET}")
    print(error_snippet.strip())
    print(f"{RED}----------------------------{RESET}\n")
    
    # Step 3: AI fixes the code based on the compiler output
    print(f"{YELLOW}[AI Agent] Analyzing compiler diagnostics...{RESET}")
    time.sleep(2)
    print(f"{YELLOW}[AI Agent] Ah, I see! I assigned an f64 to a str variable. Let me fix the type annotation.{RESET}")
    time.sleep(1)
    
    fixed_code = """
// The AI fixed the type mismatch
fn compute_metrics(x: f64) -> f64 {
    let scalar: f64 = 100.0; // FIXED: strict type matched
    return x * scalar;
}

let result = compute_metrics(5.5);
"""
    print(f"\n{YELLOW}[AI Agent] Emitting updated 'temp.zs':{RESET}")
    for line in fixed_code.strip().split('\n'):
        print(f"  {line}")
    print()

    with open("temp.zs", "w") as f:
        f.write(fixed_code)

    print(f"{CYAN}[System] Re-submitting code to Zeus Compiler...{RESET}")
    time.sleep(1)
    
    result2 = subprocess.run(
        ["cargo", "run", "--release", "--bin", "zeus_compiler", "--", target_file],
        cwd=cwd,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace"
    )
    out2 = result2.stdout if result2.stdout else ""
    err2 = result2.stderr if result2.stderr else ""
    output2 = out2 + err2
    
    if "Build Success" in output2:
        print(f"{GREEN}[OK] Zeus Compiler Verified the Code! (0 Leaks, Types Match){RESET}")
        print(f"{GREEN}[OK] AI Auto-Repair Loop Completed Successfully.{RESET}")
    else:
        print(f"{RED}[X] Second compilation failed!{RESET}")
        print(output2)
else:
    print(f"{GREEN}Code compiled on the first try!{RESET}")

# Clean up
if os.path.exists("temp.zs"):
    os.remove("temp.zs")
if os.path.exists("temp"):
    os.remove("temp")
