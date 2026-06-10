(module
  (func $relu (export "relu") (param $v i32) (result i32)
    local.get $v
    i32.const 0
    i32.gt_s
    if
    local.get $v
    return
    end
    i32.const 0
    return
    i32.const 0
  )
  (func $clampv (export "clampv") (param $v i32) (param $lo i32) (param $hi i32) (result i32)
    local.get $v
    local.get $lo
    i32.lt_s
    if
    local.get $lo
    return
    end
    local.get $v
    local.get $hi
    i32.gt_s
    if
    local.get $hi
    return
    end
    local.get $v
    return
    i32.const 0
  )
  (func $neuron4 (export "neuron4") (param $x0 i32) (param $x1 i32) (param $x2 i32) (param $x3 i32) (result i32)
    (local $acc i32)
    i32.const 0
    local.set $acc
    i32.const 2
    local.get $x0
    i32.mul
    local.get $x1
    i32.sub
    i32.const 3
    local.get $x2
    i32.mul
    i32.add
    local.get $x3
    i32.add
    local.set $acc
    local.get $acc
    call $relu
    return
    i32.const 0
  )
  (func $ramp (export "ramp") (param $n_unused i32) (result i32)
    (local $s i32)
    (local $i i32)
    i32.const 0
    local.set $s
    i32.const 0
    local.set $i
    block $brk0
    loop $cont0
    local.get $i
    i32.const 10
    i32.ge_s
    br_if $brk0
    local.get $s
    local.get $i
    i32.add
    local.set $s
    local.get $i
    i32.const 1
    i32.add
    local.set $i
    br $cont0
    end
    end
    local.get $s
    return
    i32.const 0
  )
)
