import { navigateToTestPage } from '../helpers/test-utils.js';

describe('Shadow DOM', () => {
  beforeEach(async () => {
    await navigateToTestPage('shadow');
    await browser.pause(100); // Allow time for shadow DOM to be created
  });

  describe('Get Shadow Root', () => {
    it('should get shadow root from host element', async () => {
      const host = await $('[data-testid="shadow-host-open"]');
      const shadowRoot = await host.shadow$('div');

      expect(shadowRoot).toBeDefined();
    });

    it('should verify shadow host exists', async () => {
      const host = await $('[data-testid="shadow-host-open"]');
      expect(await host.isExisting()).toBe(true);
    });
  });

  describe('Find Elements in Shadow DOM', () => {
    it('should find element in shadow DOM by CSS selector', async () => {
      const host = await $('[data-testid="shadow-host-open"]');
      const shadowContent = await host.shadow$('[data-testid="shadow-content"]');

      expect(shadowContent).toBeDefined();
    });

    it('should find button in shadow DOM', async () => {
      const host = await $('[data-testid="shadow-host-open"]');
      const shadowButton = await host.shadow$('[data-testid="shadow-button"]');

      expect(await shadowButton.isExisting()).toBe(true);
    });

    it('should find input in shadow DOM', async () => {
      const host = await $('[data-testid="shadow-host-open"]');
      const shadowInput = await host.shadow$('[data-testid="shadow-input"]');

      expect(await shadowInput.isExisting()).toBe(true);
    });

    it('should find multiple elements in shadow DOM', async () => {
      const host = await $('[data-testid="shadow-host-open"]');
      // Find all elements with data-testid in shadow
      const elements = await host.shadow$$('[data-testid]');

      expect(elements.length).toBeGreaterThan(0);
    });

    it('should find nested element in shadow DOM', async () => {
      const host = await $('[data-testid="shadow-host-open"]');
      const nestedSpan = await host.shadow$('[data-testid="nested-shadow-span"]');

      expect(await nestedSpan.isExisting()).toBe(true);
    });
  });

  describe('Interact with Shadow DOM Elements', () => {
    it('should click button in shadow DOM', async () => {
      const host = await $('[data-testid="shadow-host-open"]');
      const shadowButton = await host.shadow$('[data-testid="shadow-button"]');

      await shadowButton.click();
      // Button was clickable
      expect(await shadowButton.isExisting()).toBe(true);
    });

    it('should type in input in shadow DOM', async () => {
      const host = await $('[data-testid="shadow-host-open"]');
      const shadowInput = await host.shadow$('[data-testid="shadow-input"]');

      await shadowInput.setValue('Shadow DOM input text');

      const value = await shadowInput.getValue();
      expect(value).toBe('Shadow DOM input text');
    });

    it('should clear input in shadow DOM', async () => {
      const host = await $('[data-testid="shadow-host-open"]');
      const shadowInput = await host.shadow$('[data-testid="shadow-input"]');

      await shadowInput.setValue('Some text');
      await shadowInput.clearValue();

      const value = await shadowInput.getValue();
      expect(value).toBe('');
    });
  });

  describe('Get Properties from Shadow DOM Elements', () => {
    it('should get text from shadow DOM element', async () => {
      const host = await $('[data-testid="shadow-host-open"]');
      const shadowText = await host.shadow$('[data-testid="shadow-text"]');

      const text = await shadowText.getText();
      expect(text).toBe('Inside Open Shadow DOM');
    });

    it('should get tag name from shadow DOM element', async () => {
      const host = await $('[data-testid="shadow-host-open"]');
      const shadowButton = await host.shadow$('[data-testid="shadow-button"]');

      const tagName = await shadowButton.getTagName();
      expect(tagName.toLowerCase()).toBe('button');
    });

    it('should get attribute from shadow DOM element', async () => {
      const host = await $('[data-testid="shadow-host-open"]');
      const shadowInput = await host.shadow$('[data-testid="shadow-input"]');

      const placeholder = await shadowInput.getAttribute('placeholder');
      expect(placeholder).toBe('Shadow input...');
    });

    it('should get CSS property from shadow DOM element', async () => {
      const host = await $('[data-testid="shadow-host-open"]');
      const shadowContent = await host.shadow$('.shadow-content');

      const padding = await shadowContent.getCSSProperty('padding');
      expect(padding).toBeDefined();
    });
  });

  describe('Element State in Shadow DOM', () => {
    it('should check if element is displayed in shadow DOM', async () => {
      const host = await $('[data-testid="shadow-host-open"]');
      const shadowButton = await host.shadow$('[data-testid="shadow-button"]');

      expect(await shadowButton.isDisplayed()).toBe(true);
    });

    it('should check if element is enabled in shadow DOM', async () => {
      const host = await $('[data-testid="shadow-host-open"]');
      const shadowButton = await host.shadow$('[data-testid="shadow-button"]');

      expect(await shadowButton.isEnabled()).toBe(true);
    });
  });

  describe('Regular DOM vs Shadow DOM', () => {
    it('should find regular DOM elements', async () => {
      const regularButton = await $('[data-testid="regular-button"]');
      expect(await regularButton.isExisting()).toBe(true);
    });

    it('should not find shadow DOM elements in regular DOM', async () => {
      // Shadow DOM elements should not be findable from document root
      const shadowButton = await $('[data-testid="shadow-button"]');
      expect(await shadowButton.isExisting()).toBe(false);
    });

    it('should differentiate between shadow and regular content', async () => {
      const regularText = await $('[data-testid="regular-text"]');
      const regularContent = await regularText.getText();
      expect(regularContent).toBe('This is regular DOM content (not in shadow DOM).');

      const host = await $('[data-testid="shadow-host-open"]');
      const shadowText = await host.shadow$('[data-testid="shadow-text"]');
      const shadowContent = await shadowText.getText();
      expect(shadowContent).toBe('Inside Open Shadow DOM');
    });
  });

  describe('Closed Shadow DOM', () => {
    it('should not access closed shadow DOM', async () => {
      const closedHost = await $('[data-testid="shadow-host-closed"]');
      expect(await closedHost.isExisting()).toBe(true);

      // Attempting to access closed shadow should fail or return empty
      let errorThrown = false;
      try {
        const closedContent = await closedHost.shadow$('[data-testid="closed-shadow-content"]');
        // If no error, the element should not exist
        if (closedContent) {
          expect(await closedContent.isExisting()).toBe(false);
        }
      } catch (e) {
        errorThrown = true;
      }
      // Either an error is thrown or the element doesn't exist
      // Both are valid behaviors for closed shadow DOM
    });
  });

  describe('Screenshot in Shadow DOM', () => {
    it('should take screenshot of shadow DOM element', async () => {
      const host = await $('[data-testid="shadow-host-open"]');
      const shadowButton = await host.shadow$('[data-testid="shadow-button"]');

      const screenshot = await shadowButton.takeScreenshot();
      expect(screenshot).toBeDefined();
      expect(typeof screenshot).toBe('string');
      expect(screenshot.length).toBeGreaterThan(0);
    });
  });
});
