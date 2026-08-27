mod parser;
mod simulator;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: ...");
        return;
    }

    if let Err(e) = parser::parse(&args) {
        eprintln!("{e}")
    }
}
