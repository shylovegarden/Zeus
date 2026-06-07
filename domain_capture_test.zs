
extern fn matmul(A: tensor<10, 10>, B: tensor<10, 10>) -> tensor<10, 10>;


component struct Transform {
    x: f64,
    y: f64,
    z: f64,
}


pub fn process_secure_data() {
    enclave {
        let secret key = 1.23;
        let result = key;
    }
}


pub fn ai_matmul(A: sparse tensor<10, 10>, B: tensor<10, 10>) {
    matmul(A, B);
}
