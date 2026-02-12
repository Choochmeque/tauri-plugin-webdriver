import { navigateToTestPage } from '../helpers/test-utils.js';

describe.skip('Alerts', () => {
  beforeEach(async () => {
    await navigateToTestPage('alerts');
  });

  describe('Alert Dialog', () => {
    it('should accept alert', async () => {
      const button = await $('[data-testid="alert-button"]');
      await button.click();

      // Wait for alert to appear
      await browser.pause(100);

      // Accept the alert
      await browser.acceptAlert();

      // Verify result is updated
      const result = await $('[data-testid="alert-result"]');
      const text = await result.getText();
      expect(text).toContain('Alert was shown');
    });

    it('should dismiss alert', async () => {
      const button = await $('[data-testid="alert-button"]');
      await button.click();

      await browser.pause(100);

      // Dismiss the alert
      await browser.dismissAlert();

      // Alert should be dismissed
      const result = await $('[data-testid="alert-result"]');
      const text = await result.getText();
      expect(text).toContain('Alert was shown');
    });

    it('should get alert text', async () => {
      const button = await $('[data-testid="alert-button"]');
      await button.click();

      await browser.pause(100);

      // Get alert text
      const alertText = await browser.getAlertText();
      expect(alertText).toBe('This is a test alert message!');

      // Clean up
      await browser.acceptAlert();
    });

    it('should handle custom alert message', async () => {
      const button = await $('[data-testid="custom-alert-button"]');
      await button.click();

      await browser.pause(100);

      const alertText = await browser.getAlertText();
      expect(alertText).toBe('Custom message: Hello from WebDriver test!');

      await browser.acceptAlert();
    });
  });

  describe('Confirm Dialog', () => {
    it('should accept confirm dialog', async () => {
      const button = await $('[data-testid="confirm-button"]');
      await button.click();

      await browser.pause(100);

      // Accept (click OK)
      await browser.acceptAlert();

      const result = await $('[data-testid="confirm-result"]');
      const text = await result.getText();
      expect(text).toBe('User clicked OK');
    });

    it('should dismiss confirm dialog', async () => {
      const button = await $('[data-testid="confirm-button"]');
      await button.click();

      await browser.pause(100);

      // Dismiss (click Cancel)
      await browser.dismissAlert();

      const result = await $('[data-testid="confirm-result"]');
      const text = await result.getText();
      expect(text).toBe('User clicked Cancel');
    });

    it('should get confirm dialog text', async () => {
      const button = await $('[data-testid="confirm-button"]');
      await button.click();

      await browser.pause(100);

      const alertText = await browser.getAlertText();
      expect(alertText).toBe('Do you want to confirm this action?');

      await browser.acceptAlert();
    });
  });

  describe('Prompt Dialog', () => {
    it('should accept prompt with default value', async () => {
      const button = await $('[data-testid="prompt-button"]');
      await button.click();

      await browser.pause(100);

      // Accept without changing the value
      await browser.acceptAlert();

      const result = await $('[data-testid="prompt-result"]');
      const text = await result.getText();
      expect(text).toBe('User entered: Default Value');
    });

    it('should send text to prompt', async () => {
      const button = await $('[data-testid="prompt-button"]');
      await button.click();

      await browser.pause(100);

      // Send custom text
      await browser.sendAlertText('Custom Input');
      await browser.acceptAlert();

      const result = await $('[data-testid="prompt-result"]');
      const text = await result.getText();
      expect(text).toBe('User entered: Custom Input');
    });

    it('should dismiss prompt (cancel)', async () => {
      const button = await $('[data-testid="prompt-button"]');
      await button.click();

      await browser.pause(100);

      // Dismiss (click Cancel)
      await browser.dismissAlert();

      const result = await $('[data-testid="prompt-result"]');
      const text = await result.getText();
      expect(text).toBe('User cancelled the prompt');
    });

    it('should get prompt dialog text', async () => {
      const button = await $('[data-testid="prompt-button"]');
      await button.click();

      await browser.pause(100);

      const alertText = await browser.getAlertText();
      expect(alertText).toBe('Please enter your name:');

      await browser.dismissAlert();
    });

    it('should send empty text to prompt', async () => {
      const button = await $('[data-testid="prompt-button"]');
      await button.click();

      await browser.pause(100);

      // Send empty string
      await browser.sendAlertText('');
      await browser.acceptAlert();

      const result = await $('[data-testid="prompt-result"]');
      const text = await result.getText();
      expect(text).toBe('User entered: ');
    });

    it('should send special characters to prompt', async () => {
      const button = await $('[data-testid="prompt-button"]');
      await button.click();

      await browser.pause(100);

      // Send text with special characters
      await browser.sendAlertText('Test <>&"\'');
      await browser.acceptAlert();

      const result = await $('[data-testid="prompt-result"]');
      const text = await result.getText();
      expect(text).toContain('User entered:');
    });
  });

  describe('Alert Error Handling', () => {
    it('should handle no alert present error', async () => {
      // No alert is open, trying to accept should throw
      let errorThrown = false;
      try {
        await browser.acceptAlert();
      } catch (e) {
        errorThrown = true;
      }
      expect(errorThrown).toBe(true);
    });

    it('should handle get alert text when no alert', async () => {
      let errorThrown = false;
      try {
        await browser.getAlertText();
      } catch (e) {
        errorThrown = true;
      }
      expect(errorThrown).toBe(true);
    });
  });

  describe('Delayed Alert', () => {
    it('should handle delayed alert', async () => {
      const button = await $('[data-testid="delayed-alert-button"]');
      await button.click();

      // Wait for the delayed alert (1 second delay)
      await browser.pause(1200);

      const alertText = await browser.getAlertText();
      expect(alertText).toBe('Delayed alert!');

      await browser.acceptAlert();
    });
  });
});
