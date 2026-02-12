import { navigateToTestPage } from '../helpers/test-utils.js';

describe('Navigation', () => {
  beforeEach(async () => {
    // Navigate to main page before each test
    await navigateToTestPage('main');
  });

  describe('URL Navigation', () => {
    it('should navigate to a URL', async () => {
      await navigateToTestPage('forms');
      const url = await browser.getUrl();
      expect(url).toContain('#forms');
    });

    it('should get current URL', async () => {
      const url = await browser.getUrl();
      expect(url).toBeDefined();
      expect(typeof url).toBe('string');
      expect(url).toContain('tauri://localhost');
    });

    it('should navigate to different hash routes', async () => {
      const routes = ['forms', 'frames', 'shadow', 'alerts', 'scroll', 'main'];

      for (const route of routes) {
        await navigateToTestPage(route);
        const url = await browser.getUrl();
        expect(url).toContain(`#${route}`);
      }
    });
  });

  describe('Page Title', () => {
    it('should get page title', async () => {
      const title = await browser.getTitle();
      expect(title).toBeDefined();
      expect(typeof title).toBe('string');
      // The Tauri app title is set in index.html
      expect(title).toBe('Tauri + Svelte');
    });
  });

  describe('History Navigation', () => {
    it('should navigate back in history', async () => {
      // Navigate to forms page
      await navigateToTestPage('forms');

      // Navigate to alerts page
      await navigateToTestPage('alerts');

      // Go back
      await browser.back();
      await browser.pause(100);

      const url = await browser.getUrl();
      expect(url).toContain('#forms');
    });

    it('should navigate forward in history', async () => {
      // Navigate to forms page
      await navigateToTestPage('forms');

      // Navigate to alerts page
      await navigateToTestPage('alerts');

      // Go back
      await browser.back();
      await browser.pause(100);

      // Go forward
      await browser.forward();
      await browser.pause(100);

      const url = await browser.getUrl();
      expect(url).toContain('#alerts');
    });
  });

  describe('Page Refresh', () => {
    it('should refresh the page', async () => {
      // Navigate to forms page
      await navigateToTestPage('forms');

      // Type something in an input
      const input = await $('[data-testid="text-input"]');
      await input.setValue('test value');

      // Refresh the page
      await browser.refresh();
      await browser.pause(100);

      // The input should be cleared after refresh
      const inputAfterRefresh = await $('[data-testid="text-input"]');
      const value = await inputAfterRefresh.getValue();
      expect(value).toBe('');
    });
  });

  describe('Page Source', () => {
    it('should get page source', async () => {
      const source = await browser.getPageSource();
      expect(source).toBeDefined();
      expect(typeof source).toBe('string');
      expect(source.length).toBeGreaterThan(0);
      // Should contain HTML content
      expect(source).toContain('<');
      expect(source).toContain('>');
    });

    it('should contain expected elements in page source', async () => {
      const source = await browser.getPageSource();
      // Should contain the welcome heading
      expect(source).toContain('Welcome to Tauri!');
      expect(source).toContain('data-testid="welcome-heading"');
    });
  });
});
