# Policy Enforcement in Zeus

## Overview

Zeus supports policy enforcement via the `--policy` flag:

```bash
zeus build program.zs --policy=strict.policy
```

## Policy File Format

Create a `zeus.policy` file:

```
# Required properties
require: zero-heap, constant-time, deterministic

# Forbidden operations
forbid: malloc, free, syscall, network, file-io, random

# Compliance standards
comply: FDA-IEC62304, MISRA-C, NASA-Class-D

# Custom rules
custom: no-floating-point-in-crypto
```

## Example Policies

### Medical Device Policy
```
# IEC 62304 Class C
require: zero-heap, bounded, reproducible
forbid: malloc, free, syscall, network, file-io, random, time
comply: FDA-IEC62304

# Safety critical requirements
require: @wcet, @stack, @ensures
forbid: panic, undefined_behavior
```

### Cryptocurrency Policy
```
# Smart contract requirements
require: zero-heap, deterministic, constant-time
forbid: random, time, network, file-io
comply: constant-time-crypto

# Gas optimization
custom: prefer-inlined-functions
custom: minimize-storage-access
```

### Aerospace Policy
```
# NASA Class D
require: zero-heap, bounded, deterministic, reproducible
forbid: malloc, free, syscall, network, file-io, random, time
comply: NASA-Class-D, MISRA-C

# Real-time requirements
require: @wcet(1000), @stack(4KB)
forbid: recursion, dynamic-allocation
```

## CLI Usage

```bash
# Build with policy
echo "require: zero-heap, constant-time" > strict.policy
zeus build crypto.zs --policy=strict.policy

# Run with policy verification
echo "require: deterministic" > runtime.policy
zeus run program.zs --policy=runtime.policy

# Policy violations cause build failure
zeus build bad_program.zs --policy=strict.policy
# [POLICY VIOLATIONS]
#   - main: uses malloc (forbidden)
#   - encrypt: uses secret in branch (not constant-time)
```

## Programmatic Usage

```rust
use zeus::policy::PolicyEngine;

let engine = PolicyEngine::from_file("strict.policy")?;
engine.enforce(&program)?;

// Generate compliance report
let report = engine.generate_report();
println!("{}", report);
```

## Integration with CI/CD

```yaml
# .github/workflows/zeus.yml
- name: Verify Policy Compliance
  run: |
    zeus build src/ --policy=production.policy
    zeus cert src/main.zs
    
- name: Check Certificate
  run: |
    zeus verify-cert src/main.zcert
```

## Custom Policy Rules

```rust
// Custom rule implementation
impl PolicyEngine {
    fn check_custom(&self, rule: &str, program: &Program) {
        match rule {
            "no-floating-point-in-crypto" => {
                // Check no f64 in crypto functions
            }
            "prefer-inlined-functions" => {
                // Suggest inlining for small functions
            }
            _ => {}
        }
    }
}
```

## Policy Violations

When a policy is violated:

```
[POLICY VIOLATIONS]
  - main: uses malloc (forbidden by policy)
  - encrypt: uses secret in branch (not constant-time)
  - verify: missing @ensures (FDA-IEC62304 requires)
  
[POLICY] Build failed due to violations
```

## Best Practices

1. **Start strict, relax as needed**
2. **One policy per project type**
3. **Version control your policies**
4. **Document policy exceptions**
5. **Review policies regularly**

## Predefined Policies

Zeus includes templates for common use cases:

```bash
# Copy predefined policy
zeus policy --template=medical > medical.policy
zeus policy --template=crypto > crypto.policy
zeus policy --template=aerospace > aerospace.policy
```

## Policy Report

Generate detailed compliance reports:

```bash
zeus build program.zs --policy=strict.policy --report=policy-report.md
```

Output:
```markdown
# Policy Compliance Report

## Summary
- Required properties: 5/5 satisfied
- Forbidden operations: 0/12 violations
- Compliance standards: 3/3 met

## Details
- ✅ zero-heap: No heap allocation detected
- ✅ constant-time: No secret branches detected
- ✅ deterministic: No non-deterministic sources
- ✅ FDA-IEC62304: All requirements met
- ✅ NASA-Class-D: All requirements met
```
