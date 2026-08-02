# Tree-Sitter-mcp (WIP)

An MCP server for AI coding agents that turns a codebase into **structure**, not
text. Instead of grep-and-read loops, agents ask for exactly what they need — a
symbol and its references, a function's signature, a construct's byte range —
and get typed, compact answers.

## What is tree-sitter

[Tree-sitter](https://tree-sitter.github.io/) is an incremental, error-tolerant
parsing framework. It builds a **concrete syntax tree** from source code — the
actual `function_declaration`/`struct_specification` nodes and their named
children — fast enough to re-parse on every keystroke. I learnt about from how extensively its used in [Helix-editor](https://helix-editor.com).

The point for agents: tree-sitter understands **structure** that this
identifier is a definition and that one is a call, which statements a function
body contains, where a reference resolves.

## Why agents need it

Coding agents today navigate code via grep/read loop

1. `grep` for a symbol name → a wall of text matches, with no structure.
2. Read the whole file to understand which match is the definition.
3. `grep` again for callers → repeat, fanning out across the codebase.

That loop burns tokens and context window on noise. Every read pastes thousands
of tokens into the model, most of it irrelevant to the question being asked, and
a text-only search can't distinguish a definition from a usage, a method from a
field, or a real caller from a comment.

## How it improves agent efficiency

Think of this, how do you find definition of a function body, or the exact fields of a struct through grep?
Through guesses. The agent makes a rough estimate which results in problems underselling(not enough context) and overselling(too much context)

Tree-sitter allows to fetch exactly the required amount of information.
This hopefully(not yet tested against real workflows) improves token efficiency and drastically reduces guesstimates.

To make AI natively rely upon this, you will probably need custom agent that bypass the default harness' grep/read loop.


## Setup

use `TREE_SITTER_MCP_GRAMMAR_DIR` to set a custom grammar directory
