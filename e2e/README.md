# E2E Tests

End-to-end tests for Maudit using Playwright.

## Setup

```bash
cd e2e
pnpm install
npx playwright install
```

## Running Tests

The tests will automatically:
1. Build the prefetch.js bundle (via `cargo xtask build-maudit-js`)
2. Start the Maudit dev server on the test fixture site
3. Run the tests

```bash
# Run all tests
pnpm test

# Run tests in UI mode
pnpm test:ui

# Run tests in debug mode
pnpm test:debug

# Run tests with browser visible
pnpm test:headed

# Run tests only on Chromium (for Speculation Rules tests)
pnpm test:chromium

# Show test report
pnpm report
```

## Test Structure

- `fixtures/test-site/` - Simple Maudit site used for testing
- `tests/prefetch.spec.ts` - Tests for basic prefetch functionality
- `tests/prerender.spec.ts` - Tests for Speculation Rules prerendering

## Features Tested

### Basic Prefetch
- Creating link elements with `rel="prefetch"`
- Preventing duplicate prefetches
- Skipping current page prefetch
- Blocking cross-origin prefetches

### Prerendering (Chromium only)
- Creating `<script type="speculationrules">` elements
- Different eagerness levels (immediate, eager, moderate, conservative)
- Fallback to link prefetch on non-Chromium browsers
- Multiple URL prerendering

## Notes

- Speculation Rules API tests only run on Chromium (Chrome/Edge 109+)
- The test server runs on `http://127.0.0.1:3456`
- Tests automatically skip unsupported features on different browsers
