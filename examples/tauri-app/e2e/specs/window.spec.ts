import { navigateToTestPage, isMobile } from '../helpers/test-utils.js';

describe('Window Management', () => {
  beforeEach(async () => {
    await navigateToTestPage('main');
  });

  describe('Window Handle', () => {
    it('should get current window handle', async () => {
      const handle = await browser.getWindowHandle();

      expect(handle).toBeDefined();
      expect(typeof handle).toBe('string');
      expect(handle.length).toBeGreaterThan(0);
    });

    it('should return consistent window handle', async () => {
      const handle1 = await browser.getWindowHandle();
      const handle2 = await browser.getWindowHandle();

      expect(handle1).toBe(handle2);
    });
  });

  describe('Window Handles', () => {
    it('should get all window handles', async () => {
      const handles = await browser.getWindowHandles();

      expect(handles).toBeDefined();
      expect(Array.isArray(handles)).toBe(true);
      expect(handles.length).toBeGreaterThan(0);
    });

    it('should include current window in handles', async () => {
      const currentHandle = await browser.getWindowHandle();
      const allHandles = await browser.getWindowHandles();

      expect(allHandles).toContain(currentHandle);
    });
  });

  describe('Window Rect', () => {
    it('should get window rect', async () => {
      const rect = await browser.getWindowRect();

      expect(rect).toBeDefined();
      expect(typeof rect.x).toBe('number');
      expect(typeof rect.y).toBe('number');
      expect(typeof rect.width).toBe('number');
      expect(typeof rect.height).toBe('number');
    });

    it('should have positive dimensions', async () => {
      const rect = await browser.getWindowRect();

      expect(rect.width).toBeGreaterThan(0);
      expect(rect.height).toBeGreaterThan(0);
    });

    // Skip on mobile - window rect manipulation not supported
    (isMobile() ? it.skip : it)('should set window rect', async () => {
      const newRect = {
        x: 100,
        y: 100,
        width: 800,
        height: 600,
      };

      await browser.setWindowRect(newRect.x, newRect.y, newRect.width, newRect.height);
      await browser.pause(100);

      const rect = await browser.getWindowRect();

      // Allow some tolerance for window manager adjustments
      expect(rect.width).toBeGreaterThanOrEqual(newRect.width - 50);
      expect(rect.width).toBeLessThanOrEqual(newRect.width + 50);
      expect(rect.height).toBeGreaterThanOrEqual(newRect.height - 50);
      expect(rect.height).toBeLessThanOrEqual(newRect.height + 50);
    });

    // Skip on mobile - window rect manipulation not supported
    (isMobile() ? it.skip : it)('should set window size only', async () => {
      const initialRect = await browser.getWindowRect();

      await browser.setWindowRect(null, null, 900, 700);
      await browser.pause(100);

      const newRect = await browser.getWindowRect();

      // Size should be updated (with tolerance)
      expect(newRect.width).toBeGreaterThanOrEqual(850);
      expect(newRect.width).toBeLessThanOrEqual(950);
      expect(newRect.height).toBeGreaterThanOrEqual(650);
      expect(newRect.height).toBeLessThanOrEqual(750);
    });

    // Skip on mobile - window rect manipulation not supported
    (isMobile() ? it.skip : it)('should set window position only', async () => {
      await browser.setWindowRect(200, 150, null, null);
      await browser.pause(100);

      const rect = await browser.getWindowRect();

      // Position should be updated (with tolerance for window manager)
      expect(rect.x).toBeGreaterThanOrEqual(150);
      expect(rect.x).toBeLessThanOrEqual(250);
      expect(rect.y).toBeGreaterThanOrEqual(100);
      expect(rect.y).toBeLessThanOrEqual(200);
    });
  });

  // Skip entire section on mobile - window state manipulation not supported
  (isMobile() ? describe.skip : describe)('Window Maximize', () => {
    it('should maximize window', async () => {
      const initialRect = await browser.getWindowRect();

      await browser.maximizeWindow();
      await browser.pause(200);

      const maximizedRect = await browser.getWindowRect();

      // Maximized window should be larger or equal
      expect(maximizedRect.width).toBeGreaterThanOrEqual(initialRect.width);
      expect(maximizedRect.height).toBeGreaterThanOrEqual(initialRect.height);
    });
  });

  // Skip entire section on mobile - window state manipulation not supported
  (isMobile() ? describe.skip : describe)('Window Minimize', () => {
    it('should minimize window', async () => {
      // Minimize the window
      await browser.minimizeWindow();
      await browser.pause(200);

      // Note: Minimized window behavior varies by platform
      // Some platforms may still report the original dimensions
      // We just verify the command doesn't throw
    });

    it('should restore from minimized', async () => {
      await browser.minimizeWindow();
      await browser.pause(100);

      // Restore by setting rect
      await browser.setWindowRect(100, 100, 800, 600);
      await browser.pause(100);

      const rect = await browser.getWindowRect();
      expect(rect.width).toBeGreaterThan(0);
      expect(rect.height).toBeGreaterThan(0);
    });
  });

  // Skip entire section on mobile - window state manipulation not supported
  (isMobile() ? describe.skip : describe)('Window Fullscreen', () => {
    it('should enter fullscreen', async () => {
      const initialRect = await browser.getWindowRect();

      await browser.fullscreenWindow();
      await browser.pause(200);

      const fullscreenRect = await browser.getWindowRect();

      // Fullscreen should generally be larger or equal to initial
      expect(fullscreenRect.width).toBeGreaterThanOrEqual(initialRect.width);
      expect(fullscreenRect.height).toBeGreaterThanOrEqual(initialRect.height);
    });

    it('should exit fullscreen by setting rect', async () => {
      await browser.fullscreenWindow();
      await browser.pause(100);

      // Exit fullscreen by setting specific size
      await browser.setWindowRect(100, 100, 800, 600);
      await browser.pause(100);

      const rect = await browser.getWindowRect();
      expect(rect.width).toBeLessThanOrEqual(900);
    });
  });

  describe('Window Switching', () => {
    it('should switch to window by handle', async () => {
      const handles = await browser.getWindowHandles();

      if (handles.length > 0) {
        await browser.switchToWindow(handles[0]);
        const currentHandle = await browser.getWindowHandle();
        expect(currentHandle).toBe(handles[0]);
      }
    });
  });
});
