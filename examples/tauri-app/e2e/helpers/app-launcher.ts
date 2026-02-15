import { spawn, spawnSync, ChildProcess } from 'node:child_process';
import { resolve, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { existsSync } from 'node:fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

let appProcess: ChildProcess | null = null;
let adbPortForwarded = false;

type Platform = 'desktop' | 'android' | 'ios';

// Android app package name from tauri.conf.json identifier
const ANDROID_PACKAGE = 'test.tauri.webdriver';
const ANDROID_ACTIVITY = '.MainActivity';

function getPlatform(): Platform {
  const env = process.env.TAURI_TEST_PLATFORM;
  if (env === 'android') return 'android';
  if (env === 'ios') return 'ios';
  return 'desktop';
}

function getAdbPath(): string {
  const androidHome = process.env.ANDROID_HOME;
  if (!androidHome) {
    throw new Error('ANDROID_HOME environment variable is not set');
  }

  const adbPath = join(androidHome, 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb');
  if (!existsSync(adbPath)) {
    throw new Error(`adb not found at ${adbPath}`);
  }

  return adbPath;
}

function runAdb(args: string[]): { success: boolean; output: string } {
  const adb = getAdbPath();
  console.log(`[adb] ${args.join(' ')}`);

  const result = spawnSync(adb, args, { encoding: 'utf-8' });

  if (result.error) {
    console.error(`[adb error]: ${result.error.message}`);
    return { success: false, output: result.error.message };
  }

  const output = (result.stdout || '') + (result.stderr || '');
  if (result.status !== 0) {
    console.error(`[adb failed]: ${output}`);
    return { success: false, output };
  }

  return { success: true, output: output.trim() };
}

function setupAdbPortForward(port: number): void {
  const result = runAdb(['forward', `tcp:${port}`, `tcp:${port}`]);
  if (!result.success) {
    throw new Error(`Failed to set up adb port forwarding: ${result.output}`);
  }
  adbPortForwarded = true;
  console.log(`Port forwarding set up: localhost:${port} -> device:${port}`);
}

function removeAdbPortForward(port: number): void {
  if (!adbPortForwarded) return;

  runAdb(['forward', '--remove', `tcp:${port}`]);
  adbPortForwarded = false;
  console.log(`Port forwarding removed for port ${port}`);
}

function startAndroidApp(): void {
  const component = `${ANDROID_PACKAGE}/${ANDROID_ACTIVITY}`;
  const result = runAdb(['shell', 'am', 'start', '-n', component]);
  if (!result.success) {
    throw new Error(`Failed to start Android app: ${result.output}`);
  }
  console.log(`Started Android app: ${component}`);
}

function stopAndroidApp(): void {
  runAdb(['shell', 'am', 'force-stop', ANDROID_PACKAGE]);
  console.log(`Stopped Android app: ${ANDROID_PACKAGE}`);
}

export function getAppPath(): string {
  const base = resolve(__dirname, '../../src-tauri/target/release');

  switch (process.platform) {
    case 'darwin': {
      // Try bundled app first, fall back to unbundled binary (--no-bundle)
      const bundledPath = resolve(base, 'bundle/macos/tauri-app.app/Contents/MacOS/tauri-app');
      const unbundledPath = resolve(base, 'tauri-app');
      return existsSync(bundledPath) ? bundledPath : unbundledPath;
    }
    case 'win32':
      return resolve(base, 'tauri-app.exe');
    case 'linux':
      return resolve(base, 'tauri-app');
    default:
      throw new Error(`Unsupported platform: ${process.platform}`);
  }
}

export function getDevAppPath(): string {
  const base = resolve(__dirname, '../../src-tauri/target/debug');

  switch (process.platform) {
    case 'darwin': {
      // Try bundled app first, fall back to unbundled binary
      const bundledPath = resolve(base, 'bundle/macos/tauri-app.app/Contents/MacOS/tauri-app');
      const unbundledPath = resolve(base, 'tauri-app');
      return existsSync(bundledPath) ? bundledPath : unbundledPath;
    }
    case 'win32':
      return resolve(base, 'tauri-app.exe');
    case 'linux':
      return resolve(base, 'tauri-app');
    default:
      throw new Error(`Unsupported platform: ${process.platform}`);
  }
}

async function waitForServer(port: number, timeout: number = 30000): Promise<void> {
  const startTime = Date.now();

  while (Date.now() - startTime < timeout) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/status`);
      if (response.ok) {
        console.log(`WebDriver server ready on port ${port}`);
        return;
      }
    } catch {
      // Server not ready yet
    }
    await new Promise(resolve => setTimeout(resolve, 500));
  }

  throw new Error(`WebDriver server did not start within ${timeout}ms`);
}

export async function startApp(port: number = 4445): Promise<ChildProcess | null> {
  const platform = getPlatform();

  if (platform === 'android') {
    console.log('Setting up Android test environment...');

    // Set up port forwarding from host to device
    setupAdbPortForward(port);

    // Start the Android app
    startAndroidApp();

    // Wait for WebDriver server to be ready
    await waitForServer(port);
    return null;
  }

  if (platform === 'ios') {
    // iOS - just wait for server, user handles app lifecycle
    console.log(`Waiting for iOS app on port ${port}...`);
    await waitForServer(port);
    return null;
  }

  // Desktop - spawn app
  const appPath = getAppPath();
  console.log(`Starting Tauri app: ${appPath}`);

  appProcess = spawn(appPath, [], {
    env: {
      ...process.env,
      TAURI_WEBDRIVER_PORT: port.toString(),
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  appProcess.stdout?.on('data', (data) => {
    console.log(`[app stdout]: ${data.toString().trim()}`);
  });

  appProcess.stderr?.on('data', (data) => {
    console.error(`[app stderr]: ${data.toString().trim()}`);
  });

  appProcess.on('error', (err) => {
    console.error('Failed to start app:', err);
  });

  appProcess.on('exit', (code, signal) => {
    console.log(`App exited with code ${code}, signal ${signal}`);
    appProcess = null;
  });

  await waitForServer(port);

  return appProcess;
}

export function stopApp(port: number = 4445): void {
  const platform = getPlatform();

  if (platform === 'android') {
    console.log('Cleaning up Android test environment...');
    stopAndroidApp();
    removeAdbPortForward(port);
    return;
  }

  if (platform === 'ios') {
    // iOS - nothing to do, user handles app lifecycle
    return;
  }

  // Desktop
  if (appProcess) {
    console.log('Stopping Tauri app...');
    appProcess.kill('SIGTERM');
    appProcess = null;
  }
}

export function getAppProcess(): ChildProcess | null {
  return appProcess;
}
