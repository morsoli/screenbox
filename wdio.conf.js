export const config = {
  runner: 'local',
  logLevel: 'error',
  specs: ['./e2e/**/*.e2e.js'],
  maxInstances: 1,
  capabilities: [{ browserName: 'tauri' }],
  services: [['tauri', { appBinaryPath: './src-tauri/target/debug/screenbox', driverProvider: 'embedded' }]],
  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: { timeout: 180000 },
  waitforTimeout: 120000,
}
