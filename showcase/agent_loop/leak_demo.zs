// The agent submitted a PIN check that branches on the SECRET -- a timing leak.
// No safe automated fix exists, so the loop must ESCALATE, not certify.
@constant_time
fn pin_ok(secret pin: i32, guess: i32) -> i32 {
    if pin == guess { return 1; }
    return 0;
}
pub fn main() { println(pin_ok(1234, 1234)); }
