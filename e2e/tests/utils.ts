import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

// Find the actual prefetch bundle file (hash changes on each build)
const distDir = join(process.cwd(), "../crates/maudit/js/dist");
const prefetchFile = readdirSync(distDir).find(
	(f) => f.startsWith("prefetch") && f.endsWith(".js"),
);
if (!prefetchFile) throw new Error("Could not find prefetch bundle");

// Read the bundled prefetch script
const prefetchBundled = readFileSync(join(distDir, prefetchFile), "utf-8");

// Extract the internal function name from export{X as prefetch}
const exportMatch = prefetchBundled.match(/export\{(\w+) as prefetch\}/);
if (!exportMatch) throw new Error("Could not parse prefetch export");
const internalName = exportMatch[1];

// Remove export and expose on window
export const prefetchScript = `
	(function() {
		${prefetchBundled.replace(/export\{.*\};?$/, "")}
		window.prefetch = ${internalName};
	})();
`;
