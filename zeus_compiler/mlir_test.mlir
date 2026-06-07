module {
  func.func @gpu_kernel(%arg0: !llvm.ptr) -> () {
    %t = tensor.empty() : tensor<1024x1024xf64>
    scf.parallel (%i, %j) = (%c0, %c0) to (%c10, %c10) step (%c1, %c1) {
      // let x = arith.addf 5, 10
      scf.yield
    }
    return
  }
}
