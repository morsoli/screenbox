import { defineConfig } from 'vitest/config'

export default defineConfig({ test: { include: ['src/**/*.test.ts', 'src/**/*.test.tsx'], exclude: ['vendor/**', 'node_modules/**'] } })
