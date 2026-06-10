(module
  (func $calculate_score (export "calculate_score") (param $input i32) (result i32)
    (local $result i32)
    local.get $input
    i32.const 2
    i32.mul
    local.set $result
    local.get $result
    return
    i32.const 0
  )
  (@custom "zeus_cert" "{
  \"zeus_certificate\":\"v1\",
  \"source\":\"..\test_ai_pcc.zs\",
  \"source_sha256\":\"db2b83313651c5c8971277acc680e31e28995cd431234458715fcb362d804999\",
  \"zero_heap\":true,
  \"ffi_unaudited\":false,
  \"functions\":[
    {\"name\":\"calculate_score\",\"reproducible\":true,\"constant_time\":true,\"wcet_steps\":10,\"stack_bytes\":80,\"ffi_unaudited\":false}
  ],
  \"signature\":\"159d6cfcfebd747a33286fb75306b455f0e9361262e7a430d03ba526a9975b56b8605bac7dd5d96883dc2c18fa98f0f40ca5fe6a2edd182602c392b414252604\",
  \"pubkey\":\"faefc9f8bbf0c01668e61d05d6f6dd2a66c478940feb03b4fc82675b4de8d5a3\"
}")
)
