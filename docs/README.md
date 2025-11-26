# Consair Documentation

This directory contains the Consair documentation and GitHub Pages site.

**Live site:** [https://tsmarsh.github.io/consair/](https://tsmarsh.github.io/consair/)

## Documentation

### Language Reference
- [Language Overview](language/README.md) - Introduction to Consair Lisp
- [Data Types](language/types.md) - Numbers, strings, symbols, lists, vectors, maps, sets
- [Special Forms](language/special-forms.md) - `quote`, `if`, `cond`, `lambda`, `label`, `defmacro`
- [Standard Library](language/stdlib.md) - Built-in functions

### Tools
- [cons](tools/cons.md) - Interactive REPL and interpreter
- [cadr](tools/cadr.md) - Ahead-of-time compiler to LLVM IR

### Internals
- [Architecture](internals/architecture.md) - Interpreter, JIT, and AOT design

### Examples
- [Example Programs](examples/README.md) - Sample code and tutorials

## Site Structure

- `index.md` - Main landing page (rendered by Jekyll)
- `_config.yml` - Jekyll configuration
- `SETUP.md` - Instructions for enabling GitHub Pages and Codecov
- `language/` - Language reference documentation
- `tools/` - CLI tool documentation
- `internals/` - Architecture documentation
- `examples/` - Example programs

## Local Development

To preview locally:

```bash
# Using Python
python3 -m http.server 8000

# Using Node
npx http-server .

# Then open http://localhost:8000
```

## Features

The landing page includes:
- Hero section with download CTAs
- Feature showcase grid
- Live code examples
- Installation guide
- Responsive design
- Dark theme matching GitHub's aesthetic
- CI/Coverage badges
