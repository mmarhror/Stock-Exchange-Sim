// use std::collections::HashMap;
use std::fs;

#[derive(Default)]
pub struct Config {
    // pub stocks: HashMap<String, usize>,
    // pub processes: Vec<String>,
    // pub optimize: Vec<String>,
}

pub fn parse(args: &Vec<String>) -> Result<Config, String> {
    let file = &args[1];
    // let time = &args[2];

    parse_file(file)?;
    todo!()
}

fn parse_file(file_name: &str) -> Result<Config, String> {
    let content = fs::read_to_string(&file_name).map_err(|_| "Failed to read file")?;

    let lines: Vec<&str> = content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    for line in &lines {
        if line.starts_with("optimize") {
            println!("OPTIMIZE: {}", line);
        } else if line.contains("(") {
            println!("PROCESS: {}", line);
        } else {
            println!("STOCK: {}", line);
        }
    }

    Ok(Config {})
}

// enum LineType {
//     Stock((String, usize)),
//     Process(String),
//     Optimize(String),
// }
