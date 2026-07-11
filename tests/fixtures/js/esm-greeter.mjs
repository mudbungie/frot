// Hand-authored ES module fixture (data, not a dependency) for the golden suite:
// a relative module the inline page module imports and renders. Exercises the
// real ESM path — resolver + loader over subfetch, live module linking — that a
// module-graph page takes (js.md §4.1/§6, bl-1b98). Kept tiny and framework-free
// so the assertion is about module resolution, not a bundle's behavior.
export const GREETING = 'Hello from an ES module';

export function greet(where) {
  return GREETING + ' → ' + where;
}
