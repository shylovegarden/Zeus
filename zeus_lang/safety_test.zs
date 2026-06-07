
pub fn initialize_engine() {
    let max_temp = 90;
    proof {
        assert(max_temp LessThan 100);
        assert(max_temp LessThan 80);
    }
}
