# luacells

A Rust text-based cellular automata simulator that uses Lua for rule definitions.

## Installation

With cargo:

```bash
  cargo install luacells
```

## Usage

```bash
  luacells rules/life.lua
  luacells --help # For more information
```

You can find the controls at the bottom of the viewer.

## Rule format

Rules are given as lua programs with three globals: `Update`, `Display`, and `States`.

Example (Conway's Game of Life):

```lua
Update = function(c, n)
  local sum = 0
  for _, v in ipairs(n) do
    sum = sum + v
  end
  if c == 0 then
    if sum == 3 then
      return 1
    else
      return 0
    end
  else
    if sum == 2 or sum == 3 then
      return 1
    else
      return 0
    end
  end
end

Display = function(n)
  if n == 0 then return "  " end
  if n == 1 then return "()" end
end

States = 2
```

### `States`

`States` is simply the number of states a cell can be in.

### `Update`

`Update` is a function describing how to update a cell.

It is given two arguments:

1. The previous state of that cell
2. The previous states in the square neighborhood around the cell

The neighborhood is given as a table in this order:

1. North
2. South
3. East
4. West
5. Northeast
6. Southeast
7. Northwest
8. Southwest

### `Display`

`Display` is the function that displays a cell. 

It is given the value of the cell and should return a string of length 1 or 2.

### `Randomize`

You can optionally add `Randomize = true` to the rule file to randomize on startup.

## Pattern format

The patterns are just lists of rows of numbers.

The rows are delemited by semicolons, and the cells are delemited by commas.

## This repository

This repository contains the Rust source code along with some rules and patterns, in the `rules` and `patterns` directories.
