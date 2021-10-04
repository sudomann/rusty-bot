use rand::Rng;

pub fn coin_flip() -> String {
    let is_heads: bool = rand::thread_rng().gen();
    if is_heads {
        "Heads".to_string()
    } else {
        "Tails".to_string()
    }
}
