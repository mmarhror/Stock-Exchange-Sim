# Stock Exchange Simulator

A small Rust project for simulating resource-based production chains and finding a high-scoring schedule under an optimization goal.

The simulator reads a configuration file, explores valid process schedules, and chooses the best result according to an `optimize:` directive such as maximizing a stock, minimizing time, or balancing both.

## What it does

- Defines initial stock levels
- Parses production processes with resource consumption and output
- Starts processes when their required ingredients are available
- Advances time as running jobs complete
- Searches for the best schedule within a user-defined wall-clock limit
- Prints the final action history and final stock values
- Writes a `.log` file containing the chosen schedule

## Project structure

- `src/parser.rs` — config parsing, validation, and log parsing
- `src/simulator.rs` — state model, move generation, search algorithm, and output generation
- `src/main.rs` — CLI entry point
- `examples/` — example configuration files for different scenarios

## Configuration format

Each configuration file can contain:

- stock definitions: `name:quantity`
- process definitions: `name:(need:qty;...):(result:qty;...):delay`
- optimization directive: `optimize:(target1;target2;...)`
- comments beginning with `#`
- blank lines are ignored

### Example

```text
# initial resources
wood:10
euro:5

# process: name:(needs):(results):delay
cut_tree:(wood:2):(plank:1):5
build_table:(plank:4;euro:1):(table:1):20

optimize:(time;table)
```

### Process grammar

```text
<name>:(<need>:<qty>;...):(<result>:<qty>;...):<delay>
```

Examples:

```text
buy_material:(euro:8):(material:1):10
build_product:(material:1):(product:1):30
delivery:(product:1):(client_content:1):20
```

## Optimization rules

The `optimize:` line accepts stock names and the special target `time`.

- stock targets are compared by larger quantity being better
- `time` is treated as a tie-breaker, where lower time is preferred
- the simulator searches for the schedule that best meets the configured priorities

Example:

```text
optimize:(time;cabinet)
```

This means:

1. prefer more `cabinet` stock
2. if tied, prefer shorter completion time

## Running the simulator

From the project root:

```bash
cargo run -- <config_file> <waiting_time>
```

Example:

```bash
cargo run -- examples/simple/simple 5
```

This reads the config, simulates until the search deadline, prints the final schedule, and writes a log file next to the config.

## Example configs

The repository includes a few sample scenarios:

- `examples/simple/simple` — basic production chain
- `examples/build/build` — furniture/cabinet optimization example
- `examples/seller/seller` — resource trading / sales scenario
- `examples/fertilizer/fertilizer` — recursive growth and happiness optimization

## Notes

- The search is deadline-based: it keeps exploring until the real-world time limit is reached.
- The simulator prioritizes valid resource usage and deterministic ordering of start actions within a cycle.
- Output is intended to be human-readable, while the generated `.log` file is suitable for downstream checking or validation.
