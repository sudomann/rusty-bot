use rand::Rng;

pub async fn coin_flip() -> Result<String, ()> {
    let is_heads: bool = rand::thread_rng().gen();
    if is_heads {
        Ok("Heads".to_string())
    } else {
        Ok("Tails".to_string())
    }
}
