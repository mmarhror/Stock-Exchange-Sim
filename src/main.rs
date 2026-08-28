mod parser;
mod simulator;

use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    let (config, waiting_time) = match parser::parse(&args) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    let filename = &args[1];
    if let Err(e) = simulator::simulate(config, filename, waiting_time) {
        eprintln!("{}", e);
        process::exit(1);
    }
}
