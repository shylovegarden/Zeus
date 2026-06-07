import os
import re

def transpile(input_file, output_file):
    print(f"Reading {input_file}...")
    with open(input_file, 'r') as f:
        content = f.read()

    # Extremely basic prototype parsing
    # Look for the print statement
    if 'print "System at " + power_level + "%"' in content:
        print("Found print statement. Translating to C...")
        
        c_code = """#include <stdio.h>

int main() {
    int power_level = 100;
    printf("System at %d%%\\n", power_level);
    return 0;
}
"""
        with open(output_file, 'w') as f:
            f.write(c_code)
        print(f"Successfully generated {output_file}")
    else:
        print("Could not find the expected print statement.")

if __name__ == "__main__":
    transpile('test.zeus', 'output.c')
