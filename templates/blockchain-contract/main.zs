// Blockchain Contract Template - Provable Gas Bounds
@smart_contract
@evm_target
@gas_bound(100000)
@deterministic

struct Account {
    balance: u64,
    nonce: u64
}

@gas_bound(50000)
pub fn transfer(from: Account, to: Account, amount: u64) -> (Account, Account) {
    @requires(from.balance >= amount)
    @requires(amount > 0)
    @ensures(from.balance == old(from.balance) - amount)
    @ensures(to.balance == old(to.balance) + amount)
    
    let mut new_from = from;
    let mut new_to = to;
    
    new_from.balance = from.balance - amount;
    new_from.nonce = from.nonce + 1;
    new_to.balance = to.balance + amount;
    
    return (new_from, new_to);
}

pub fn main() {
    let alice = Account { balance: 1000, nonce: 0 };
    let bob = Account { balance: 500, nonce: 0 };
    let (new_alice, new_bob) = transfer(alice, bob, 100);
    println(new_alice.balance);
}
