use std::collections::HashMap;
use std::sync::OnceLock;
use std::fs;

use regex::Regex;

#[derive(Debug, Clone)]
pub struct Config {
    pub stocks: HashMap<String, usize>,
    pub processes: Vec<Process>,
    pub optimize: Vec<String>,
}

fn get_file_lines(file_name: &str) -> Result<Vec<String>, String> {
    let content = fs
        ::read_to_string(file_name)
        .map_err(|_| format!("Failed to read file: {}", file_name))?;

    let lines: Vec<String> = content
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    Ok(lines)
}

pub fn parse_file(file_name: &str) -> Result<Config, String> {
    let content = fs::read_to_string(&file_name).map_err(|_| "Failed to read file")?;

    let lines: Vec<&str> = content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    let mut stocks: HashMap<String, usize> = HashMap::new();
    let mut processes: Vec<Process> = Vec::new();
    let mut optimize: Vec<String> = Vec::new();

    for line in &lines {
        if line.starts_with("optimize:") {
            //
            if !optimize.is_empty() {
                return Err("Duplicate optimize directive found".to_string());
            }
            optimize = parse_optimize(line)?;
            //
        } else if line.contains("(") {
            //
            let proc = parse_process(line)?;
            if processes.iter().any(|p| p.name == proc.name) {
                return Err(format!("Duplicate process definition found: {}", proc.name));
            }
            processes.push(proc);
            //
        } else {
            let (name, qty) = parse_stock(line)?;
            if stocks.insert(name.clone(), qty).is_some() {
                return Err(format!("Duplicate stock definition found: {}", name));
            }
        }
    }

    if processes.is_empty() {
        return Err("Missing processes".to_string());
    }

    Ok(Config {
        stocks,
        processes,
        optimize,
    })
}

pub fn parse_log(file_name: &str) -> Result<Vec<(usize, String)>, String> {
    let lines = get_file_lines(file_name)?;
    let mut actions: Vec<(usize, String)> = Vec::new();

    for line in lines {
        let (cycle_str, proc_name) = line
            .split_once(':')
            .ok_or_else(|| format!("Invalid log line: {}", line))?;

        let cycle: usize = cycle_str
            .parse()
            .map_err(|_| format!("Invalid cycle number: {}", line))?;

        actions.push((cycle, proc_name.to_string()));
    }

    Ok(actions)
}

pub fn parse_time(time_str: &str) -> Result<f64, String> {
    time_str.parse().map_err(|_| "Waiting time must be a valid positive integer\n".to_string())
}

// ===== Stock =====
static STOCK_RE: OnceLock<Regex> = OnceLock::new();

fn stock_regex() -> &'static Regex {
    STOCK_RE.get_or_init(|| { Regex::new(r"^(?<name>[a-zA-Z0-9_-]+):(?<qty>\d+)$").unwrap() })
}

fn stock_err(line: &str, reason: &str) -> String {
    format!("Error in stock `{}`: {}\nUsage: <stock_name>:<quantity>", line.trim(), reason)
}

fn parse_stock(line: &str) -> Result<(String, usize), String> {
    let reg = stock_regex();
    let caps = reg.captures(line.trim()).ok_or_else(|| stock_err(line, ""))?;

    let name: String = caps["name"].to_owned();
    let qty: usize = caps["qty"].parse().map_err(|_| stock_err(line, "failed to parse quantity"))?;

    Ok((name, qty))
}

// ===== Optimize =====

static OPTIMIZE_RE: OnceLock<Regex> = OnceLock::new();

fn optimize_regex() -> &'static Regex {
    OPTIMIZE_RE.get_or_init(|| { Regex::new(r"^optimize:\((?<targets>[^)]+)\)$").unwrap() })
}

fn optimize_err(line: &str, reason: &str) -> String {
    format!("Error in optimize `{}`: {}\nUsage: optimize:(<stock_name>|time)", line.trim(), reason)
}

fn parse_optimize_targets(targets_str: &str) -> Vec<String> {
    targets_str
        .split(|c| c == '|' || c == ';')
        .filter(|el| !el.is_empty())
        .map(|el| el.to_string())
        .collect()
}

fn parse_optimize(line: &str) -> Result<Vec<String>, String> {
    let reg = optimize_regex();
    let caps = reg.captures(line.trim()).ok_or_else(|| optimize_err(line, ""))?;

    let targets_str = &caps["targets"];
    let targets: Vec<String> = parse_optimize_targets(targets_str);

    Ok(targets)
}

// ===== Process =====

#[derive(Debug, Clone)]
pub struct Process {
    pub name: String,
    pub needs: HashMap<String, usize>,
    pub results: HashMap<String, usize>,
    pub delay: usize,
}

impl Process {
    pub fn can_start(&self, stocks: &HashMap<String, usize>) -> bool {
        for (name, qty) in &self.needs {
            match stocks.get(name) {
                Some(curr_qty) if curr_qty >= qty => {}
                _ => {
                    return false;
                }
            }
        }
        true
    }

    pub fn start(&self, stocks: &mut HashMap<String, usize>) {
        for (name, qty) in &self.needs {
            let curr_qty = stocks.get_mut(name).unwrap();
            *curr_qty -= qty;
            if *curr_qty == 0 {
                stocks.remove(name);
            }
        }
    }

    pub fn finish(&self, stocks: &mut HashMap<String, usize>) {
        for (name, qty) in &self.results {
            let curr_qty = stocks.entry(name.clone()).or_insert(0);
            *curr_qty += qty;
        }
    }
}

static PROCESS_RE: OnceLock<Regex> = OnceLock::new();

fn process_regex() -> &'static Regex {
    PROCESS_RE.get_or_init(|| {
        Regex::new(
            r"^(?<name>[^:]+):\((?<needs>[^)]*)\):\((?<results>[^)]*)\):(?<delay>\d+)$"
        ).unwrap()
    })
}

fn process_err(line: &str, reason: &str) -> String {
    format!(
        "Error in process `{}`: {}\nUsage: <name>:(<need>:<quantity>;...):(<result>:<quantity>;...):<delay>",
        line.trim(),
        reason
    )
}

fn parse_items(items_str: &str) -> Result<HashMap<String, usize>, String> {
    let mut items: HashMap<String, usize> = HashMap::new();

    if items_str.is_empty() {
        return Ok(items);
    }

    for item in items_str.split(";") {
        let (name, quantity) = parse_stock(item)?;

        items.insert(name, quantity);
    }

    Ok(items)
}

fn parse_process(line: &str) -> Result<Process, String> {
    let reg = process_regex();

    let caps = reg.captures(line).ok_or_else(|| process_err(line, ""))?;

    let needs_str = &caps["needs"].trim();
    let results_str = &caps["results"].trim();

    let name = caps["name"].to_string();
    let needs: HashMap<String, usize> = parse_items(needs_str)?;
    let results: HashMap<String, usize> = parse_items(results_str)?;
    let delay: usize = caps["delay"]
        .parse()
        .map_err(|_| process_err(line, "failed to parse delay"))?;

    Ok(Process {
        name,
        needs,
        results,
        delay,
    })
}
