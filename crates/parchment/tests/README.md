# Parchment WASM Tests

This directory contains interactive HTML test files for validating Parchment's WebAssembly bindings.

## Running the Tests

1. **Build the WASM package** (from the project root):
   ```bash
   wasm-pack build --target web --out-dir pkg
   ```

2. **Start a local HTTP server** (from the project root):
   ```bash
   # Using Python 3
   python -m http.server 3000
   
   # Using Python 2
   python -m SimpleHTTPServer 3000
   
   # Using Node.js
   npx http-server -p 3000
   
   # Using Bun
   bun --hot . --port 3000
   ```

3. **Open in browser**:
   ```
   http://localhost:3000/tests/test_attributor.html
   ```

## Test Files

### `test_attributor.html`
Comprehensive test suite for the Attributor WASM bindings, including:

- **Constructor Patterns**: Tests all four builder pattern constructors
  - `new(attr_name, key_name)` - Basic constructor
  - `newWithScope(attr_name, key_name, scope)` - With scope specification
  - `newWithWhitelist(attr_name, key_name, whitelist)` - With value validation
  - `newFull(attr_name, key_name, scope, whitelist)` - Full configuration

- **DOM Manipulation**: Tests core attributor functionality
  - `add(element, value)` - Setting attribute values
  - `remove(element)` - Removing attributes
  - `value(element)` - Getting current values
  - `keys(element)` - Listing all attributes

- **Validation**: Tests whitelist and scope functionality
  - Valid value acceptance
  - Invalid value rejection
  - Scope enum integration

- **Error Handling**: Tests edge cases and error conditions

## Expected Results

All tests should pass with green checkmarks (✅). Any red X marks (❌) indicate issues that need investigation.

The tests provide real-time feedback and demonstrate the full functionality of the Attributor WASM bindings in a browser environment.