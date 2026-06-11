# Tutorial 1: Getting Started with Zeus

**Time:** 5 minutes  
**Prerequisites:** Basic command line knowledge

## What You'll Learn
- How to install Zeus
- How to write your first Zeus program
- How to build and run it

## Step 1: Install Zeus

```bash
# Install using the official installer
curl -sSL https://zeus-lang.org/install.sh | bash

# Or install from source
git clone https://github.com/zeus-lang/zeus.git
cd zeus/zeus_compiler
cargo build --release
```

## Step 2: Create Your First Program

Create a file named `hello.zs`:

```zeus
pub fn main() {
    println("Hello, Zeus!");
}
```

## Step 3: Build and Run

```bash
# Build the program
zeus build hello.zs

# Run the compiled binary
./hello
```

You should see:
```
Hello, Zeus!
```

## Understanding the Code

- `pub fn main()` - The entry point of your program
- `println()` - Built-in function to print to console
- No semicolons needed (Zeus uses newlines)

## Next Steps

Try modifying the program to print your name:

```zeus
pub fn main() {
    println("Hello, Alice!");
}
```

## Troubleshooting

**Error: "zeus: command not found"**
- Make sure `$HOME/.zeus/bin` is in your PATH
- Or use the full path: `~/.zeus/bin/zeus`

**Error: "LLVM not found"**
- Install LLVM 14: `brew install llvm` (macOS) or `apt install llvm-14` (Ubuntu)

## Summary

✅ You installed Zeus  
✅ You wrote your first program  
✅ You built and ran it  

Next: [Tutorial 2: First Verification](./02-first-verification.md)
