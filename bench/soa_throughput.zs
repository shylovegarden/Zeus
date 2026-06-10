// =============================================================================
// Zeus SoA throughput benchmark
//
// `let p = Body[N]` is lowered by the Zeus compiler into N separate, 32-byte
// aligned field arrays (p_x[], p_y[], p_vx[], p_vy[]) -- a Structure-of-Arrays
// transform -- and the hot loop body is emitted as straight-line C over those
// arrays. With gcc -O3 -march=native the aligned, unit-stride access pattern
// auto-vectorizes (AVX2: 4 doubles / instruction).
//
// NOTE: `zeus build` itself compiles the emitted C at -O0, so vectorization
// only appears when the emitted .c is recompiled at -O3 (see run_bench.sh).
// This file exists so the harness can diff the Zeus-emitted SoA C against the
// hand-written naive AoS C counterpart.
// =============================================================================

struct Body {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
}

pub fn main() {
    let bodies = Body[131072];

    // Seed
    let mut i: i32 = 0;
    while i < 131072 {
        bodies[i].x = 1;
        bodies[i].y = 2;
        bodies[i].vx = 3;
        bodies[i].vy = 4;
        i = i + 1;
    }

    // Hot loop: integrate position by velocity (SAXPY-like, unit stride).
    let mut step: i32 = 0;
    while step < 512 {
        let mut j: i32 = 0;
        while j < 131072 {
            bodies[j].x = bodies[j].x + bodies[j].vx;
            bodies[j].y = bodies[j].y + bodies[j].vy;
            j = j + 1;
        }
        step = step + 1;
    }

    println(bodies[0].x);
}
