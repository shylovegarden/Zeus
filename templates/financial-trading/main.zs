// Financial Trading Template - Guaranteed Latency
@trading_system
@max_latency(10us)
@deterministic
@zero_heap

struct Order {
    price: f64,
    quantity: u64,
    side: i32  // 0=buy, 1=sell
}

struct TradeResult {
    executed: bool,
    avg_price: f64,
    filled_quantity: u64
}

@trading_system
@wcet(500)
@max_latency(10us)
pub fn match_order(order: Order, book: [Order; 100]) -> TradeResult {
    @requires(order.price > 0.0)
    @requires(order.quantity > 0)
    @ensures(!result.executed implies result.filled_quantity == 0)
    
    let mut result: TradeResult;
    result.executed = false;
    result.avg_price = 0.0;
    result.filled_quantity = 0;
    
    let mut total_cost: f64 = 0.0;
    let mut remaining: u64 = order.quantity;
    
    let mut i: i32 = 0;
    while i < 100 && remaining > 0 {
        if can_match(order, book[i]) {
            let fill_qty = min(remaining, book[i].quantity);
            total_cost = total_cost + book[i].price * fill_qty as f64;
            remaining = remaining - fill_qty;
            result.filled_quantity = result.filled_quantity + fill_qty;
        }
        i = i + 1;
    }
    
    if result.filled_quantity > 0 {
        result.executed = true;
        result.avg_price = total_cost / result.filled_quantity as f64;
    }
    
    return result;
}

fn can_match(order: Order, entry: Order) -> bool {
    return order.side != entry.side && order.price >= entry.price;
}

fn min(a: u64, b: u64) -> u64 {
    return if a < b { a } else { b };
}

pub fn main() {
    let order = Order { price: 100.0, quantity: 10, side: 0 };
    let book: [Order; 100];
    let result = match_order(order, book);
    println(if result.executed { 1 } else { 0 });
}
