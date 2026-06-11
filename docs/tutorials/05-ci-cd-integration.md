# Tutorial 5: CI/CD Integration

**Time:** 10 minutes  
**Prerequisites:** [Tutorial 4: Zero-Heap Systems](./04-zero-heap-systems.md)

## What You'll Learn
- How to use Zeus in GitHub Actions
- How to enforce security policies in CI/CD
- How to fail builds on verification failures

## Why CI/CD Integration?

Security policies are only useful if they're enforced. By adding Zeus to your CI/CD pipeline, you:
- Block insecure code before it reaches production
- Ensure every commit meets security standards
- Generate certificates for auditors automatically

## Step 1: Create a Zeus Project

Create a new repository with this structure:

```
my-secure-project/
├── src/
│   └── crypto.zs
├── .github/
│   └── workflows/
│       └── zeus-verify.yml
└── README.md
```

Create `src/crypto.zs`:

```zeus
@zero_heap
@constant_time
pub fn verify_signature(data: [u8; 32], sig: [u8; 64]) -> bool {
    let mut diff: i32 = 0;
    let mut i: i32 = 0;
    while i < 32 {
        diff = diff | ((data[i] ^ sig[i]) as i32);
        i = i + 1;
    }
    i = 32;
    while i < 64 {
        diff = diff | ((0 ^ sig[i]) as i32);
        i = i + 1;
    }
    return diff == 0;
}
```

## Step 2: Add GitHub Action

Create `.github/workflows/zeus-verify.yml`:

```yaml
name: Zeus Security Verification

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  verify:
    runs-on: ubuntu-latest
    
    steps:
    - name: Checkout code
      uses: actions/checkout@v4
    
    - name: Install Zeus
      run: |
        curl -sSL https://zeus-lang.org/install.sh | bash
        echo "$HOME/.zeus/bin" >> $GITHUB_PATH
    
    - name: Verify Security Policies
      run: |
        zeus verify --policy=zero-heap,constant-time src/crypto.zs
    
    - name: Generate Certificate
      run: |
        zeus build --cert src/crypto.zs
    
    - name: Upload Certificate
      uses: actions/upload-artifact@v4
      with:
        name: security-certificate
        path: '*.zcert'
```

## Step 3: Test the Workflow

Push your code to GitHub:

```bash
git init
git add .
git commit -m "Add Zeus verification"
git push origin main
```

Go to your GitHub repository → Actions tab. You should see the "Zeus Security Verification" workflow running.

## Understanding the Results

### ✅ Passing Build

If your code passes verification:
- Build is green ✓
- Certificate is generated
- PR can be merged

### ❌ Failing Build

If your code fails verification:
- Build is red ✗
- PR is blocked
- Error message shows what policy was violated

## Example: Deliberate Failure

Modify `src/crypto.zs` to break the policy:

```zeus
@zero_heap
@constant_time
pub fn bad_function() {
    // This will fail!
    let ptr = malloc(100);
}
```

Push the change. The build should fail with:
```
❌ Verification failed
   Policy: zero_heap
   Issue: malloc call detected
```

## Advanced: Multiple Files

For projects with multiple Zeus files:

```yaml
- name: Verify all files
  run: |
    for file in src/*.zs; do
      echo "Verifying $file..."
      zeus verify --policy=zero-heap,constant-time "$file"
    done
```

## Advanced: Strict Mode

Fail on any warning:

```yaml
- name: Strict Verification
  run: |
    zeus verify --policy=zero-heap,constant-time --strict src/
```

## Alternative: Docker

Use the Docker image for consistency:

```yaml
- name: Verify with Docker
  run: |
    docker run -v $PWD:/workspace \
      zeuslang/compiler verify \
      --policy=zero-heap,constant-time \
      /workspace/src/crypto.zs
```

## GitLab CI

For GitLab users, here's the equivalent `.gitlab-ci.yml`:

```yaml
zeus-verify:
  image: zeuslang/compiler
  script:
    - zeus verify --policy=zero-heap,constant-time src/
  artifacts:
    paths:
      - '*.zcert'
```

## Team Workflow

### For Developers
1. Write Zeus code locally
2. Run `zeus verify` before committing
3. Fix any issues
4. Push and create PR

### For Code Reviewers
1. Check CI is green
2. Review certificate if provided
3. Approve if all policies pass

### For Security Auditors
1. Download certificates from CI artifacts
2. Verify signatures
3. Confirm properties match requirements

## Exercise: Add to Your Project

1. Take an existing project (or create a new one)
2. Add a Zeus file with security policies
3. Add the GitHub Action workflow
4. Push and verify the build passes
5. Try breaking the policy and see it fail

## Troubleshooting

**"zeus: command not found"**
- Check the install step ran successfully
- Verify `$HOME/.zeus/bin` is in PATH

**"Verification timeout"**
- Complex functions may need more time
- Add `--timeout=5000` (5 seconds)

**"Certificate not found"**
- Ensure `zeus build --cert` ran successfully
- Check the artifact path matches

## Summary

✅ You set up GitHub Actions  
✅ You enforced security policies in CI  
✅ You understand team workflows  
✅ You can troubleshoot issues  

**Key Takeaway**: CI/CD enforcement makes security policies automatic and non-negotiable.

## Next Steps

- Learn more about [Zeus policies](../policies.md)
- Explore [advanced verification](../advanced.md)
- Check out [example projects](../examples.md)

---

You've completed all 5 tutorials! You're now ready to build secure, verified systems with Zeus.
