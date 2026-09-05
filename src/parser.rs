//! Configuration and Log Parser
//!
//! This module handles reading, sanitizing, and validating both the simulation
//! configuration files and the simulation output log files. It also defines
//! the `Process` and `Config` data structures.
//!
//! # Grammar & Formats
//!
//! - **Stocks:** `<name>:<quantity>`
//! - **Processes:** `<name>:(<need>:<qty>;...):(<result>:<qty>;...):<delay>`
//! - **Optimization:** `optimize:(<target>;...)`
//! - **Logs:** `<cycle>:<process_name>`

use std::collections::HashMap;
use std::fs;
use std::sync::OnceLock;
use regex::Regex;

// ---------------------------------------------------------------------------
// Struct Definitions
// ---------------------------------------------------------------------------

/// The global configuration parsed from the config file.
#[derive(Debug, Clone)]
pub struct Config {
    /// Initial quantities of available stocks.
    pub stocks: HashMap<String, usize>,
    /// All defined processes available for the simulation.
    pub processes: Vec<Process>,
    /// Priority list of targets to optimize during simulation.
    pub optimize: Vec<String>,
}

/// Representation of a single task/process in the simulation.
#[derive(Debug, Clone)]
pub struct Process {
    pub name: String,
    /// Stocks consumed when this process starts.
    pub needs: HashMap<String, usize>,
    /// Stocks produced when this process finishes.
    pub results: HashMap<String, usize>,
    /// The number of cycles this process takes to execute.
    pub delay: usize,
}

impl Process {
    /// Checks if the current stocks are sufficient to start this process.
    pub fn can_start(&self, stocks: &HashMap<String, usize>) -> bool {
        for (name, qty) in &self.needs {
            match stocks.get(name) {
                Some(&curr_qty) if curr_qty >= *qty => {}
                _ => {
                    return false;
                }
            }
        }
        true
    }

    /// Consumes the required input resources from the stock pile.
    ///
    /// # Panics
    /// Panics if the stocks do not have enough quantities to start (use `can_start` first).
    pub fn start(&self, stocks: &mut HashMap<String, usize>) {
        for (name, qty) in &self.needs {
            let curr_qty = stocks.get_mut(name).expect("checked stock missing during start");
            *curr_qty -= qty;
            if *curr_qty == 0 {
                stocks.remove(name);
            }
        }
    }

    /// Adds the process's produced results back into the stock pile.
    pub fn finish(&self, stocks: &mut HashMap<String, usize>) {
        for (name, qty) in &self.results {
            *stocks.entry(name.clone()).or_insert(0) += qty;
        }
    }
}

// ---------------------------------------------------------------------------
// Main Parsers
// ---------------------------------------------------------------------------

/// Reads a file and returns sanitized, non-empty, non-comment lines.
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

/// Parses the entire configuration file, validating syntax and semantic constraints.
pub fn parse_file(file_name: &str) -> Result<Config, String> {
    // ✨ CHANGED — Reused `get_file_lines` here to eliminate duplicate reading logic!
    let lines = get_file_lines(file_name)?;

    let mut stocks: HashMap<String, usize> = HashMap::new();
    let mut processes: Vec<Process> = Vec::new();
    let mut optimize: Vec<String> = Vec::new();

    for line in &lines {
        if line.starts_with("optimize:") {
            if !optimize.is_empty() {
                return Err("Duplicate optimize directive found".to_string());
            }
            optimize = parse_optimize(line)?;
        } else if line.contains('(') {
            let proc = parse_process(line)?;
            if processes.iter().any(|p| p.name == proc.name) {
                return Err(format!("Duplicate process definition found: {}", proc.name));
            }
            processes.push(proc);
        } else {
            let (name, qty) = parse_stock(line)?;
            if stocks.insert(name.clone(), qty).is_some() {
                return Err(format!("Duplicate stock definition found: {}", name));
            }
        }
    }

    // --- Semantic Validations ---
    if processes.is_empty() {
        return Err("Missing processes".to_string());
    }

    // ✨ NEW — Validates that there is an optimization goal to follow
    if optimize.is_empty() {
        return Err("Missing optimization directive (e.g. optimize:(time;cabinet))".to_string());
    }

    Ok(Config {
        stocks,
        processes,
        optimize,
    })
}

/// Parses a generated log file into a chronological list of action pairs: `(cycle, process_name)`.
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

/// Parses the CLI waiting time string into an `f64` representing seconds.
pub fn parse_time(time_str: &str) -> Result<f64, String> {
    time_str.parse().map_err(|_| "Waiting time must be a valid positive float number\n".to_string())
}

// ---------------------------------------------------------------------------
// Stock Parsers
// ---------------------------------------------------------------------------

static STOCK_RE: OnceLock<Regex> = OnceLock::new();

fn stock_regex() -> &'static Regex {
    STOCK_RE.get_or_init(|| { Regex::new(r"^(?<name>[a-zA-Z0-9_-]+):(?<qty>\d+)$").unwrap() })
}

fn stock_err(line: &str, reason: &str) -> String {
    let suffix = if reason.is_empty() { "".to_string() } else { format!(": {reason}") };
    format!("Error in stock `{}`{}\nUsage: <stock_name>:<quantity>", line.trim(), suffix)
}

/// Parses a single stock string (e.g., `euro:10`) into its name and quantity.
fn parse_stock(line: &str) -> Result<(String, usize), String> {
    let reg = stock_regex();
    let caps = reg.captures(line.trim()).ok_or_else(|| stock_err(line, "malformed line"))?;

    let name: String = caps["name"].to_owned();
    let qty: usize = caps["qty"].parse().map_err(|_| stock_err(line, "failed to parse quantity"))?;

    Ok((name, qty))
}

// ---------------------------------------------------------------------------
// Optimize Parsers
// ---------------------------------------------------------------------------

static OPTIMIZE_RE: OnceLock<Regex> = OnceLock::new();

fn optimize_regex() -> &'static Regex {
    OPTIMIZE_RE.get_or_init(|| { Regex::new(r"^optimize:\((?<targets>[^)]+)\)$").unwrap() })
}

fn optimize_err(line: &str, reason: &str) -> String {
    let suffix = if reason.is_empty() { "".to_string() } else { format!(": {reason}") };
    format!("Error in optimize `{}`{}\nUsage: optimize:(<stock_name>|time)", line.trim(), suffix)
}

/// Splits the optimization string target elements.
fn parse_optimize_targets(targets_str: &str) -> Vec<String> {
    // Splits by either semicolon `;` or pipe `|`
    targets_str
        .split(|c| c == '|' || c == ';')
        .filter(|el| !el.is_empty())
        .map(|el| el.to_string())
        .collect()
}

/// Parses the optimize directive line (e.g., `optimize:(time;cabinet)`) into target components.
fn parse_optimize(line: &str) -> Result<Vec<String>, String> {
    let reg = optimize_regex();
    let caps = reg.captures(line.trim()).ok_or_else(|| optimize_err(line, "malformed expression"))?;

    let targets_str = &caps["targets"];
    let targets: Vec<String> = parse_optimize_targets(targets_str);

    Ok(targets)
}

// ---------------------------------------------------------------------------
// Process Parsers
// ---------------------------------------------------------------------------

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

/// Helper to parse inner needs or results groups (e.g., `material:1;euro:2`) into a Map.
fn parse_items(items_str: &str) -> Result<HashMap<String, usize>, String> {
    let mut items: HashMap<String, usize> = HashMap::new();

    if items_str.is_empty() {
        return Ok(items);
    }

    // ✨ CHANGED — Splitting on character ';' is faster and more idiomatic than splitting on string ";"
    for item in items_str.split(';') {
        let (name, quantity) = parse_stock(item)?;
        items.insert(name, quantity);
    }

    Ok(items)
}

/// Parses a single process declaration line into a `Process` struct.
fn parse_process(line: &str) -> Result<Process, String> {
    let reg = process_regex();
    let caps = reg.captures(line).ok_or_else(|| process_err(line, "malformed line"))?;

    let needs_str = caps["needs"].trim();
    let results_str = caps["results"].trim();

    let name = caps["name"].to_string();
    let needs: HashMap<String, usize> = parse_items(needs_str)?;
    let results: HashMap<String, usize> = parse_items(results_str)?;
    let delay: usize = caps["delay"]
        .parse()
        .map_err(|_| process_err(line, "failed to parse delay"))?;

    if delay == 0 {
        return Err(process_err(line, "process delay must be greater than 0"));
    }

    Ok(Process {
        name,
        needs,
        results,
        delay,
    })
}
