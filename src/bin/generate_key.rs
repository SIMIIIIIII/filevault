use base64::{Engine, engine::general_purpose::STANDARD};
use rand::RngCore;

fn main() {
    let mut octets = [0u8; 32]; // 256 bits
    rand::thread_rng().fill_bytes(&mut octets);
    let cle_b64 = STANDARD.encode(octets);
    println!("export JWT_SECRET={cle_b64}");
}
