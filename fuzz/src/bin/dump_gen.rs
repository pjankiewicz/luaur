use std::io::Read;
fn main() {
    let mut d = Vec::new();
    std::io::stdin().read_to_end(&mut d).unwrap();
    print!("{}", luaur_fuzz::generate(&d));
}
