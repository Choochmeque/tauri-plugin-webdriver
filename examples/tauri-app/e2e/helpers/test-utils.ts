import { browser, $ } from '@wdio/globals';

export const WEBDRIVER_PORT = 4445;
export const BASE_URL = 'tauri://localhost';

export async function resetAppState(): Promise<void> {
  // Navigate to main page using click
  await navigateToTestPage('main');

  // Clear cookies
  await browser.deleteAllCookies();

  // Clear local and session storage
  await browser.execute(() => {
    localStorage.clear();
    sessionStorage.clear();
  });
}

export async function navigateToTestPage(page: string): Promise<void> {
  // Use click-based navigation instead of browser.url() for hash routes
  // browser.url() with tauri:// scheme doesn't work on Windows WebView2
  const navLink = await browser.$(`[data-testid="nav-${page}"]`);
  await navLink.click();
  // Wait for route to load
  await browser.pause(100);
}

export function generateTestId(prefix: string): string {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).substring(7)}`;
}

export async function waitForElement(selector: string, timeout: number = 5000): Promise<WebdriverIO.Element> {
  const element = await $(selector);
  await element.waitForExist({ timeout });
  return element;
}

export async function waitForElementVisible(selector: string, timeout: number = 5000): Promise<WebdriverIO.Element> {
  const element = await $(selector);
  await element.waitForDisplayed({ timeout });
  return element;
}

export async function getElementByTestId(testId: string): Promise<WebdriverIO.Element> {
  return $(`[data-testid="${testId}"]`);
}

export async function waitForTestId(testId: string, timeout: number = 5000): Promise<WebdriverIO.Element> {
  return waitForElement(`[data-testid="${testId}"]`, timeout);
}

export async function takeScreenshotAsBase64(): Promise<string> {
  return browser.takeScreenshot();
}

export function isValidBase64Png(base64String: string): boolean {
  try {
    const buffer = Buffer.from(base64String, 'base64');
    // PNG magic bytes: 89 50 4E 47 0D 0A 1A 0A
    const pngMagic = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
    return buffer.subarray(0, 8).equals(pngMagic);
  } catch {
    return false;
  }
}

export function isValidBase64Pdf(base64String: string): boolean {
  try {
    const buffer = Buffer.from(base64String, 'base64');
    // PDF magic bytes: %PDF
    const pdfMagic = Buffer.from('%PDF');
    return buffer.subarray(0, 4).equals(pdfMagic);
  } catch {
    return false;
  }
}
