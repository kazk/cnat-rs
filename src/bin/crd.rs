// Output CRD
// cargo run --bin crd
use cnat::At;

fn main() {
    println!("{}", serde_yaml::to_string(&At::crd()).unwrap())
}
