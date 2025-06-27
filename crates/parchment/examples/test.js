// Node.js/Bun test script for Parchment advanced text operations
import { readFile } from "fs/promises";

async function testParchment() {
  try {
    // Load the WASM module
    const wasmBuffer = await readFile("./pkg/quillai_parchment_bg.wasm");

    // Import the JS bindings
    const {
      test_selection_management,
      test_find_replace_operations,
      test_text_statistics,
      test_text_traversal,
      default: init,
    } = await import("./pkg/quillai_parchment.js");

    // Initialize the WASM module
    await init(wasmBuffer);

    console.log("🚀 WASM module loaded successfully!\n");

    // Run tests
    const tests = [
      { name: "Selection Management", fn: test_selection_management },
      { name: "Find/Replace Operations", fn: test_find_replace_operations },
      { name: "Text Statistics", fn: test_text_statistics },
      { name: "Text Traversal", fn: test_text_traversal },
    ];

    let passed = 0;
    let failed = 0;

    for (const test of tests) {
      try {
        console.log(`Running ${test.name} test...`);
        const result = test.fn();
        if (result) {
          console.log(`✅ ${test.name}: PASSED`);
          passed++;
        } else {
          console.log(`❌ ${test.name}: FAILED`);
          failed++;
        }
      } catch (error) {
        console.log(`💥 ${test.name}: ERROR - ${error.message}`);
        failed++;
      }
    }

    console.log(`\n📊 Test Results: ${passed} passed, ${failed} failed`);

    if (failed === 0) {
      console.log("🎉 All tests passed!");
    } else {
      console.log("⚠️  Some tests failed. Check the implementation.");
    }
  } catch (error) {
    console.error("Failed to load or run tests:", error);
  }
}

testParchment();
