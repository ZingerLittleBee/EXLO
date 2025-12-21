#!/usr/bin/env bun
/**
 * Generate runtime dependencies from built server files
 * Usage: bun scripts/gen-runtime-deps.ts
 *
 * This script analyzes the built server files to extract external dependencies
 * and generates docker/web.runtime.json with the correct versions.
 */

import { readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

const DIST_SERVER = 'apps/web/dist/server'
const WEB_PKG = 'apps/web/package.json'
const DB_PKG = 'packages/db/package.json'
const OUTPUT = 'docker/web.runtime.json'

// Read all JS files recursively
function getJsFiles(dir: string): string[] {
  const files: string[] = []
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry)
    if (statSync(path).isDirectory()) {
      files.push(...getJsFiles(path))
    } else if (entry.endsWith('.js')) {
      files.push(path)
    }
  }
  return files
}

// Extract package names from imports
function extractPackages(content: string): Set<string> {
  const packages = new Set<string>()
  // Match: from "package" or from "@scope/package"
  const regex = /from\s+["'](@?[a-zA-Z0-9_-]+(?:\/[a-zA-Z0-9_-]+)?)/g
  let match: RegExpExecArray | null
  while ((match = regex.exec(content)) !== null) {
    const pkg = match[1]
    // Skip relative imports and node: imports
    if (!(pkg.startsWith('.') || pkg.startsWith('node:'))) {
      // Get base package name (e.g., @tanstack/react-router from @tanstack/react-router/ssr/server)
      const parts = pkg.startsWith('@') ? pkg.split('/').slice(0, 2) : pkg.split('/').slice(0, 1)
      packages.add(parts.join('/'))
    }
  }
  return packages
}

// Get version from package.json
function getVersions(pkgPath: string): Record<string, string> {
  const pkg = JSON.parse(readFileSync(pkgPath, 'utf-8'))
  return { ...pkg.dependencies, ...pkg.devDependencies }
}

// Main
const jsFiles = getJsFiles(DIST_SERVER)
const allPackages = new Set<string>()

for (const file of jsFiles) {
  const content = readFileSync(file, 'utf-8')
  for (const pkg of extractPackages(content)) {
    allPackages.add(pkg)
  }
}

// Get versions from package.json files
const webVersions = getVersions(WEB_PKG)
const dbVersions = getVersions(DB_PKG)
const allVersions = { ...dbVersions, ...webVersions }

// Essential packages that may not be detected from imports
const essentialPackages = ['react', 'react-dom', 'pg', 'dotenv', 'better-auth']

// Build runtime dependencies
const runtimeDeps: Record<string, string> = {}

// Add essential packages first
for (const pkg of essentialPackages) {
  const version = allVersions[pkg]
  if (version && !version.startsWith('workspace:') && version !== 'catalog:') {
    runtimeDeps[pkg] = version
  }
}

for (const pkg of Array.from(allPackages).sort()) {
  // Skip workspace packages, devtools, and node built-ins
  if (
    pkg.startsWith('@exlo/') ||
    pkg.includes('devtools') ||
    ['crypto', 'dns', 'events', 'fs', 'net', 'node', 'path', 'stream', 'string_decoder', 'tls', 'util'].includes(pkg)
  )
    continue

  const version = allVersions[pkg]
  if (version && !version.startsWith('workspace:') && version !== 'catalog:') {
    runtimeDeps[pkg] = version
  } else if (!(essentialPackages.includes(pkg) || version)) {
    // Only warn for packages we couldn't find versions for (not built-ins)
    console.warn(`Warning: No version found for ${pkg}`)
  }
}

// Write output
const output = {
  name: 'exlo-web-runtime',
  private: true,
  dependencies: runtimeDeps
}

writeFileSync(OUTPUT, JSON.stringify(output, null, 2) + '\n')
console.log(`Generated ${OUTPUT} with ${Object.keys(runtimeDeps).length} dependencies`)
console.log(Object.keys(runtimeDeps).join(', '))
