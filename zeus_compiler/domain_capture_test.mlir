module {
  func.func private @matmul(tensor<?x?xf64>, tensor<?x?xf64>) -> tensor<?x?xf64>
  // MLIR Struct: Transform
  func.func @process_secure_data(%arg0: !llvm.ptr) -> () {
    // ENCLAVE BLOCK
      // let key = 1.23
      // let result = %key
    return
  }
  func.func @ai_matmul(%arg0: !llvm.ptr) -> () {
    // [ZEUS MLIR: SPARSE CSR FORMAT ENABLED. ZERO-PATH SKIPPING ACTIVE]
affine.for %i = 0 to %M {
  %row_start = affine.load %row_ptrs[%i] : memref<?xi32>
  %row_end = affine.load %row_ptrs[%i + 1] : memref<?xi32>
  affine.for %idx = %row_start to %row_end {
    %k = affine.load %col_indices[%idx] : memref<?xi32>
    %a = affine.load %values[%idx] : memref<?xf64>
    affine.for %j = 0 to %N {
      %b = affine.load %B[%k, %j] : memref<?x?xf64>
      %c = affine.load %out[%i, %j] : memref<?x?xf64>
      %prod = arith.mulf %a, %b : f64
      %sum = arith.addf %c, %prod : f64
      affine.store %sum, %out[%i, %j] : memref<?x?xf64>
    }
  }
}
    return
  }
}
