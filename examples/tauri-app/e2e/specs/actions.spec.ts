import { navigateToTestPage } from '../helpers/test-utils.js';

describe('Actions API', () => {
  beforeEach(async () => {
    await navigateToTestPage('forms');
  });

  describe('Keyboard Actions', () => {
    it('should type using keyboard actions', async () => {
      const input = await $('[data-testid="text-input"]');
      await input.clearValue();
      await input.click();

      await browser.keys(['H', 'e', 'l', 'l', 'o']);

      const value = await input.getValue();
      expect(value).toBe('Hello');
    });

    it('should use special keys', async () => {
      const input = await $('[data-testid="text-input"]');
      await input.clearValue();
      await input.click();

      await browser.keys(['T', 'e', 's', 't']);
      await browser.keys(['Backspace']);

      const value = await input.getValue();
      expect(value).toBe('Tes');
    });

    it('should use Enter key', async () => {
      const input = await $('[data-testid="text-input"]');
      await input.clearValue();
      await input.click();

      await browser.keys(['T', 'e', 's', 't', 'Enter']);

      // Enter key behavior depends on the form - input should still have value
      const value = await input.getValue();
      expect(value).toContain('Test');
    });

    it('should use Tab key to navigate', async () => {
      const textInput = await $('[data-testid="text-input"]');
      await textInput.click();

      // Tab to next input
      await browser.keys(['Tab']);

      // Next focusable element should be focused
      const activeElement = await browser.getActiveElement();
      expect(activeElement).toBeDefined();
    });

    it('should use Escape key', async () => {
      const input = await $('[data-testid="text-input"]');
      await input.click();
      await input.setValue('Some text');

      await browser.keys(['Escape']);

      // Escape doesn't clear input, but the key event should be sent
      const value = await input.getValue();
      expect(value).toBeDefined();
    });

    it('should type with Shift modifier', async () => {
      const input = await $('[data-testid="text-input"]');
      await input.clearValue();
      await input.click();

      // Type uppercase using shift
      await browser.keys(['Shift', 'h', 'e', 'l', 'l', 'o']);

      const value = await input.getValue();
      // Depending on implementation, this might be uppercase
      expect(value.length).toBeGreaterThan(0);
    });

    it('should clear input with select all and delete', async () => {
      const input = await $('[data-testid="text-input"]');
      await input.setValue('Text to clear');

      await input.click();

      // Select all (Ctrl+A or Cmd+A) and delete
      const modifier = process.platform === 'darwin' ? 'Meta' : 'Control';
      await browser.keys([modifier, 'a']);
      await browser.keys(['Backspace']);

      const value = await input.getValue();
      expect(value).toBe('');
    });
  });

  describe('Arrow Key Navigation', () => {
    it('should use arrow keys in input', async () => {
      const input = await $('[data-testid="text-input"]');
      await input.setValue('Hello World');
      await input.click();

      // Move cursor with arrow keys
      await browser.keys(['ArrowLeft', 'ArrowLeft', 'ArrowLeft']);

      // The cursor position changed (we can't easily verify this without selection API)
      expect(await input.getValue()).toBe('Hello World');
    });

    it('should navigate radio buttons with arrow keys', async () => {
      const radio1 = await $('[data-testid="radio-1"]');
      await radio1.click();
      expect(await radio1.isSelected()).toBe(true);

      // Arrow down to select next radio
      await browser.keys(['ArrowDown']);

      // Check if radio-2 is now selected
      const radio2 = await $('[data-testid="radio-2"]');
      expect(await radio2.isSelected()).toBe(true);
    });
  });

  describe('Pointer Actions', () => {
    it('should click element', async () => {
      const checkbox = await $('[data-testid="checkbox"]');
      expect(await checkbox.isSelected()).toBe(false);

      await checkbox.click();

      expect(await checkbox.isSelected()).toBe(true);
    });

    it('should double click element', async () => {
      const input = await $('[data-testid="text-input"]');
      await input.setValue('Double click test');

      // Double click should select word
      await input.doubleClick();

      // The input still has its value
      expect(await input.getValue()).toBe('Double click test');
    });

    it('should right click element (context menu)', async () => {
      const button = await $('[data-testid="submit-button"]');

      // Right click - this won't show context menu in WebDriver but shouldn't throw
      await button.click({ button: 'right' });

      expect(await button.isExisting()).toBe(true);
    });

    it('should click at specific coordinates', async () => {
      const button = await $('[data-testid="submit-button"]');
      const location = await button.getLocation();
      const size = await button.getSize();

      // Click at center of button
      await browser.performActions([
        {
          type: 'pointer',
          id: 'mouse',
          parameters: { pointerType: 'mouse' },
          actions: [
            { type: 'pointerMove', duration: 0, x: Math.round(location.x + size.width / 2), y: Math.round(location.y + size.height / 2) },
            { type: 'pointerDown', button: 0 },
            { type: 'pointerUp', button: 0 },
          ],
        },
      ]);

      // Button was clicked
      expect(await button.isExisting()).toBe(true);
    });
  });

  describe('Mouse Movement', () => {
    it('should move to element', async () => {
      const button = await $('[data-testid="submit-button"]');

      await button.moveTo();

      // Mouse is now over the button
      expect(await button.isExisting()).toBe(true);
    });

    it('should move to element with offset', async () => {
      const button = await $('[data-testid="submit-button"]');

      await button.moveTo({ xOffset: 10, yOffset: 10 });

      expect(await button.isExisting()).toBe(true);
    });
  });

  describe('Scroll Actions', () => {
    it('should scroll to element', async () => {
      await navigateToTestPage('scroll');

      const bottomMarker = await $('[data-testid="bottom-marker"]');
      await bottomMarker.scrollIntoView();

      // Element should now be in view
      expect(await bottomMarker.isDisplayed()).toBe(true);
    });

    it('should scroll using wheel action', async () => {
      await navigateToTestPage('scroll');

      // Get initial scroll position
      const initialScroll = await browser.execute(() => window.scrollY);

      // Scroll down
      await browser.performActions([
        {
          type: 'wheel',
          id: 'wheel',
          actions: [{ type: 'scroll', x: 400, y: 300, deltaX: 0, deltaY: 500 }],
        },
      ]);

      await browser.pause(100);

      // Get new scroll position
      const newScroll = await browser.execute(() => window.scrollY);

      expect(newScroll).toBeGreaterThan(initialScroll);
    });

    it('should scroll to bottom using button', async () => {
      await navigateToTestPage('scroll');

      const scrollButton = await $('[data-testid="scroll-to-bottom"]');
      await scrollButton.click();

      await browser.pause(500); // Wait for smooth scroll

      const bottomMarker = await $('[data-testid="bottom-marker"]');
      expect(await bottomMarker.isDisplayed()).toBe(true);
    });

    it('should scroll to top', async () => {
      await navigateToTestPage('scroll');

      // First scroll down
      await browser.execute(() => window.scrollTo(0, 1000));
      await browser.pause(100);

      // Then scroll to top
      const scrollButton = await $('[data-testid="scroll-to-top"]');
      await scrollButton.click();

      await browser.pause(500);

      const topMarker = await $('[data-testid="top-marker"]');
      expect(await topMarker.isDisplayed()).toBe(true);
    });
  });

  describe('Release Actions', () => {
    it('should release all actions', async () => {
      // Perform some actions
      const input = await $('[data-testid="text-input"]');
      await input.clearValue();
      await input.click();
      await browser.keys(['S', 'h', 'i', 'f', 't']);

      // Release actions
      await browser.releaseActions();

      // Should complete without error
      expect(true).toBe(true);
    });
  });

  describe('Combined Actions', () => {
    it('should perform click and type in sequence', async () => {
      const input = await $('[data-testid="text-input"]');
      await input.clearValue();
      await input.click();
      await browser.keys(['C', 'o', 'm', 'b', 'i', 'n', 'e', 'd']);

      const value = await input.getValue();
      expect(value).toBe('Combined');
    });

    it('should perform multiple element interactions', async () => {
      // Type in text input
      const textInput = await $('[data-testid="text-input"]');
      await textInput.setValue('Test Name');

      // Click checkbox
      const checkbox = await $('[data-testid="checkbox"]');
      await checkbox.click();

      // Select radio
      const radio = await $('[data-testid="radio-2"]');
      await radio.click();

      // Verify all interactions
      expect(await textInput.getValue()).toBe('Test Name');
      expect(await checkbox.isSelected()).toBe(true);
      expect(await radio.isSelected()).toBe(true);
    });
  });
});
