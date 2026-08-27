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
        if line.starts_with("optimize:") {
            println!("OPTIMIZE: {}", line);
        } else if line.contains("(") {
            println!("PROCESS: {}", line);
        } else {
            println!("STOCK: {}", line);
        }
    }

    Ok(Config {})
}

// ===== Stock =====
fn stock_err(line: &str, reason: &str) -> String {
    format!("Error in stock `{}`: {}\nUsage: <stock_name>:<quantity>", line.trim(), reason)
}

fn parse_stock(line: &str) -> Result<(String, usize), String> {
    let parts: Vec<&str> = line.trim().split(":").collect();

    if parts.len() != 2 {
        return Err(stock_err(line, "expected exactly one colon ':' between name and quantity"));
    }

    let name = parts[0].trim();
    let quantity: usize = parts[1]
        .trim()
        .parse()
        .map_err(|_| stock_err(line, "quantity must be a valid positive integer"))?;

    Ok((name.to_owned(), quantity))
}

// ===== Optimize =====

fn optimize_err(line: &str, reason: &str) -> String {
    format!("Error in optimize `{}`: {}\nUsage: optimize:(<stock_name>|time)", line.trim(), reason)
}

fn parse_optimize(line: &str) -> Result<Vec<String>, String> {
    let mut to_op = line.trim().strip_prefix("optimize:").unwrap().trim();

    if !to_op.starts_with("(") || !to_op.ends_with(")") {
        return Err(optimize_err(line, "expected targets wrapped in parentheses `(...)`"));
    }

    to_op = &to_op[1..to_op.len() - 1];

    let elems: Vec<String> = to_op
        .split(|ch| (ch == ';' || ch == '|'))
        .filter(|elem| !elem.is_empty())
        .map(|elem| elem.to_string())
        .collect();

    if elems.is_empty() {
        return Err(optimize_err(line, "no valid optimization targets found"));
    }

    Ok(elems)
}

// ===== Optimize =====

fn process_err(line: &str, reason: &str) -> String {
    format!(
        "Error in process `{}`: {}\nUsage: <name>:(<need>:<quantity>;...):(<result>:<quantity>;...):<delay>",
        line.trim(),
        reason
    )
}

fn parse_process(line: &str) -> String {
    
}
