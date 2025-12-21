# Extract runtime dependencies from package.json
# Filters out workspace:, catalog:, and build-time only packages
# Adds required workspace package dependencies
#
# When to modify this file:
# - Adding normal npm runtime dependency (e.g. "lodash": "^4.0.0") → No changes needed
# - Adding build-time dependency → Add to is_build_only list below
# - Adding catalog: reference → Add fixed version to dependencies at bottom
# - Adding workspace: reference → Add its runtime deps to dependencies at bottom

# Build-time only packages (not needed at runtime)
def is_build_only:
  . as $pkg |
  [
    "@tailwindcss/vite",
    "@tanstack/react-start",
    "@tanstack/router-plugin",
    "@tanstack/react-router-with-query",
    "@tanstack/react-query",
    "tailwindcss",
    "vite-tsconfig-paths",
    "tw-animate-css"
  ] | any(. == $pkg);

{
  name: "exlo-web-runtime",
  private: true,
  dependencies: (
    .dependencies
    | to_entries
    | map(select(
        (.value | type == "string") and
        ((.value | startswith("workspace:")) | not) and
        ((.value | startswith("catalog:")) | not) and
        ((.key | is_build_only) | not)
      ))
    | from_entries
  ) + {
    "drizzle-orm": "^0.45.1",
    "pg": "^8.16.3",
    "better-auth": "^1.4.7",
    "dotenv": "^17.2.3",
    "zod": "^4.2.1"
  }
}
