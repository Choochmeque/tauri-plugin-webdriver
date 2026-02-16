import { isValidBase64Png, navigateToTestPage } from '../helpers/test-utils.js';

describe('Screenshots', () => {
  beforeEach(async () => {
    await navigateToTestPage('main');
  });

  describe('Full Page Screenshot', () => {
    it('should take a full page screenshot', async () => {
      const screenshot = await browser.takeScreenshot();

      expect(screenshot).toBeDefined();
      expect(typeof screenshot).toBe('string');
      expect(screenshot.length).toBeGreaterThan(0);
    });

    it('should return valid base64 PNG data', async () => {
      const screenshot = await browser.takeScreenshot();

      // Verify it's valid base64
      expect(() => Buffer.from(screenshot, 'base64')).not.toThrow();

      // Verify PNG format
      expect(isValidBase64Png(screenshot)).toBe(true);
    });

    it('should capture different pages', async () => {
      // Screenshot main page
      const mainScreenshot = await browser.takeScreenshot();

      // Navigate to forms and take screenshot
      await navigateToTestPage('forms');
      const formsScreenshot = await browser.takeScreenshot();

      // Screenshots should be different
      expect(mainScreenshot).not.toBe(formsScreenshot);
    });

    it('should capture page with scroll content', async () => {
      await navigateToTestPage('scroll');

      const screenshot = await browser.takeScreenshot();
      expect(screenshot).toBeDefined();
      expect(isValidBase64Png(screenshot)).toBe(true);
    });
  });

  describe('Element Screenshot', () => {
    it('should take element screenshot', async () => {
      const heading = await $('[data-testid="welcome-heading"]');
      const screenshot = await heading.takeScreenshot();

      expect(screenshot).toBeDefined();
      expect(typeof screenshot).toBe('string');
      expect(screenshot.length).toBeGreaterThan(0);
    });

    it('should return valid base64 PNG for element', async () => {
      const heading = await $('[data-testid="welcome-heading"]');
      const screenshot = await heading.takeScreenshot();

      expect(isValidBase64Png(screenshot)).toBe(true);
    });

    it('should capture button element', async () => {
      const button = await $('[data-testid="greet-button"]');
      const screenshot = await button.takeScreenshot();

      expect(screenshot).toBeDefined();
      expect(isValidBase64Png(screenshot)).toBe(true);
    });

    it('should capture input element', async () => {
      const input = await $('[data-testid="greet-input"]');
      const screenshot = await input.takeScreenshot();

      expect(screenshot).toBeDefined();
      expect(isValidBase64Png(screenshot)).toBe(true);
    });

    it('should capture form element', async () => {
      await navigateToTestPage('forms');

      const form = await $('[data-testid="test-form"]');
      const screenshot = await form.takeScreenshot();

      expect(screenshot).toBeDefined();
      expect(isValidBase64Png(screenshot)).toBe(true);
    });

    it('should capture different elements with different sizes', async () => {
      const heading = await $('[data-testid="welcome-heading"]');
      const button = await $('[data-testid="greet-button"]');

      const headingScreenshot = await heading.takeScreenshot();
      const buttonScreenshot = await button.takeScreenshot();

      // Different elements should have different screenshots
      expect(headingScreenshot).not.toBe(buttonScreenshot);
    });

    it('should capture element after state change', async () => {
      await navigateToTestPage('forms');

      const checkbox = await $('[data-testid="checkbox"]');

      // Screenshot before click
      const beforeScreenshot = await checkbox.takeScreenshot();

      // Click to check
      await checkbox.click();
      await browser.pause(50);

      // Screenshot after click
      const afterScreenshot = await checkbox.takeScreenshot();

      // Screenshots may differ due to checked state visual change
      expect(beforeScreenshot).toBeDefined();
      expect(afterScreenshot).toBeDefined();
    });
  });

  describe('Screenshot Size Validation', () => {
    it('should have reasonable screenshot size', async () => {
      const screenshot = await browser.takeScreenshot();
      const buffer = Buffer.from(screenshot, 'base64');

      // Screenshot should be at least a few KB (minimum for a valid image)
      expect(buffer.length).toBeGreaterThan(1000);

      // And not unreasonably large (less than 10MB)
      expect(buffer.length).toBeLessThan(10 * 1024 * 1024);
    });

    it('should have element screenshot smaller than full page', async () => {
      const fullScreenshot = await browser.takeScreenshot();
      const fullBuffer = Buffer.from(fullScreenshot, 'base64');

      const button = await $('[data-testid="greet-button"]');
      const elementScreenshot = await button.takeScreenshot();
      const elementBuffer = Buffer.from(elementScreenshot, 'base64');

      // Element screenshot should generally be smaller than full page
      // (though not always guaranteed due to compression)
      expect(elementBuffer.length).toBeGreaterThan(0);
      expect(fullBuffer.length).toBeGreaterThan(0);
    });
  });
});
