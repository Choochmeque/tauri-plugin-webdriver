import { navigateToTestPage } from '../helpers/test-utils.js';

/**
 * Clicks an element asynchronously and waits for an alert to appear.
 * This triggers the click via setTimeout so that the execute() call returns
 * before the alert blocks, allowing WebDriver to interact with the alert.
 */
async function clickAndWaitForAlert(selector: string): Promise<void> {
  await browser.execute((sel: string) => {
    setTimeout(() => {
      const el = document.querySelector(sel) as HTMLElement;
      el?.click();
    }, 10);
  }, selector);

  // Wait for alert to be captured by the delegate
  await browser.waitUntil(
    async () => {
      try {
        await browser.getAlertText();
        return true;
      } catch {
        return false;
      }
    },
    { timeout: 2000, interval: 50 }
  );
}

describe('Alerts', () => {
  beforeEach(async () => {
    await navigateToTestPage('alerts');
  });

  describe('Alert Dialog', () => {
    it('should accept alert', async () => {
      await clickAndWaitForAlert('[data-testid="alert-button"]');

      // Accept the alert
      await browser.acceptAlert();

      // Verify result is updated
      const result = await $('[data-testid="alert-result"]');
      const text = await result.getText();
      expect(text).toContain('Alert was shown');
    });

    it('should dismiss alert', async () => {
      await clickAndWaitForAlert('[data-testid="alert-button"]');

      // Dismiss the alert
      await browser.dismissAlert();

      // Alert should be dismissed
      const result = await $('[data-testid="alert-result"]');
      const text = await result.getText();
      expect(text).toContain('Alert was shown');
    });

    it('should get alert text', async () => {
      await clickAndWaitForAlert('[data-testid="alert-button"]');

      // Get alert text
      const alertText = await browser.getAlertText();
      expect(alertText).toBe('This is a test alert message!');

      // Clean up
      await browser.acceptAlert();
    });

    it('should handle custom alert message', async () => {
      await clickAndWaitForAlert('[data-testid="custom-alert-button"]');

      const alertText = await browser.getAlertText();
      expect(alertText).toBe('Custom message: Hello from WebDriver test!');

      await browser.acceptAlert();
    });
  });

  describe('Confirm Dialog', () => {
    it('should accept confirm dialog', async () => {
      await clickAndWaitForAlert('[data-testid="confirm-button"]');

      // Accept (click OK)
      await browser.acceptAlert();

      const result = await $('[data-testid="confirm-result"]');
      const text = await result.getText();
      expect(text).toBe('User clicked OK');
    });

    it('should dismiss confirm dialog', async () => {
      await clickAndWaitForAlert('[data-testid="confirm-button"]');

      // Dismiss (click Cancel)
      await browser.dismissAlert();

      const result = await $('[data-testid="confirm-result"]');
      const text = await result.getText();
      expect(text).toBe('User clicked Cancel');
    });

    it('should get confirm dialog text', async () => {
      await clickAndWaitForAlert('[data-testid="confirm-button"]');

      const alertText = await browser.getAlertText();
      expect(alertText).toBe('Do you want to confirm this action?');

      await browser.acceptAlert();
    });
  });

  describe('Prompt Dialog', () => {
    it('should accept prompt with default value', async () => {
      await clickAndWaitForAlert('[data-testid="prompt-button"]');

      // Accept without changing the value
      await browser.acceptAlert();

      const result = await $('[data-testid="prompt-result"]');
      const text = await result.getText();
      expect(text).toBe('User entered: Default Value');
    });

    it('should send text to prompt', async () => {
      await clickAndWaitForAlert('[data-testid="prompt-button"]');

      // Send custom text
      await browser.sendAlertText('Custom Input');
      await browser.acceptAlert();

      const result = await $('[data-testid="prompt-result"]');
      const text = await result.getText();
      expect(text).toBe('User entered: Custom Input');
    });

    it('should dismiss prompt (cancel)', async () => {
      await clickAndWaitForAlert('[data-testid="prompt-button"]');

      // Dismiss (click Cancel)
      await browser.dismissAlert();

      const result = await $('[data-testid="prompt-result"]');
      const text = await result.getText();
      expect(text).toBe('User cancelled the prompt');
    });

    it('should get prompt dialog text', async () => {
      await clickAndWaitForAlert('[data-testid="prompt-button"]');

      const alertText = await browser.getAlertText();
      expect(alertText).toBe('Please enter your name:');

      await browser.dismissAlert();
    });

    it('should send empty text to prompt', async () => {
      await clickAndWaitForAlert('[data-testid="prompt-button"]');

      // Send empty string
      await browser.sendAlertText('');
      await browser.acceptAlert();

      const result = await $('[data-testid="prompt-result"]');
      const text = await result.getText();
      expect(text).toBe('User entered: ');
    });

    it('should send special characters to prompt', async () => {
      await clickAndWaitForAlert('[data-testid="prompt-button"]');

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
      await browser.execute(() => {
        setTimeout(() => {
          const el = document.querySelector('[data-testid="delayed-alert-button"]') as HTMLElement;
          el?.click();
        }, 10);
      });

      // Wait for the delayed alert (has ~1 second delay in the app)
      await browser.waitUntil(
        async () => {
          try {
            await browser.getAlertText();
            return true;
          } catch {
            return false;
          }
        },
        { timeout: 3000, interval: 100 }
      );

      const alertText = await browser.getAlertText();
      expect(alertText).toBe('Delayed alert!');

      await browser.acceptAlert();
    });
  });
});
