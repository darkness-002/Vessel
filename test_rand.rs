use rand::RngCore;
fn main() {
    let mut key = [0u8; 32];
    rand::rng().fill_bytes(&mut key);
    println!("Works");
}
