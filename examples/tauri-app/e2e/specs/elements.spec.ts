import { navigateToTestPage } from '../helpers/test-utils.js';

describe('Element Operations', () => {
  beforeEach(async () => {
    await navigateToTestPage('main');
  });

  describe('Finding Elements', () => {
    describe('CSS Selector', () => {
      it('should find element by CSS selector', async () => {
        const element = await $('[data-testid="welcome-heading"]');
        expect(await element.isExisting()).toBe(true);
      });

      it('should find element by ID', async () => {
        const element = await $('#greet-input');
        expect(await element.isExisting()).toBe(true);
      });

      it('should find element by class', async () => {
        const element = await $('.container');
        expect(await element.isExisting()).toBe(true);
      });

      it('should find multiple elements', async () => {
        const elements = await $$('[data-testid^="nav-"]');
        expect(elements.length).toBeGreaterThan(0);
      });
    });

    describe('XPath', () => {
      it('should find element by XPath', async () => {
        const element = await $('//h1[@data-testid="welcome-heading"]');
        expect(await element.isExisting()).toBe(true);
      });

      it('should find element by XPath with text', async () => {
        const element = await $('//h1[contains(text(), "Welcome")]');
        expect(await element.isExisting()).toBe(true);
      });

      it('should find multiple elements by XPath', async () => {
        const elements = await $$('//a[@data-testid]');
        expect(elements.length).toBeGreaterThan(0);
      });
    });

    describe('Tag Name', () => {
      it('should find element by tag name', async () => {
        const element = await $('<h1>');
        expect(await element.isExisting()).toBe(true);
      });

      it('should find multiple elements by tag name', async () => {
        const elements = await $$('<button>');
        expect(elements.length).toBeGreaterThan(0);
      });
    });

    describe('Link Text', () => {
      it('should find link by exact text', async () => {
        const element = await $('=Click Here For Link One');
        expect(await element.isExisting()).toBe(true);
      });

      it('should find link by partial text', async () => {
        const element = await $('*=Partial Link');
        expect(await element.isExisting()).toBe(true);
      });
    });

    describe('Child Elements', () => {
      it('should find child element from parent', async () => {
        const parent = await $('[data-testid="greet-section"]');
        const child = await parent.$('[data-testid="greet-input"]');
        expect(await child.isExisting()).toBe(true);
      });

      it('should find multiple child elements', async () => {
        const parent = await $('[data-testid="nav"]');
        const children = await parent.$$('a');
        expect(children.length).toBeGreaterThan(0);
      });
    });

    describe('Active Element', () => {
      it('should get active element', async () => {
        const input = await $('[data-testid="greet-input"]');
        await input.click();

        const activeElement = await browser.getActiveElement();
        expect(activeElement).toBeDefined();
      });
    });

    describe('Element Not Found', () => {
      it('should handle non-existent element', async () => {
        const element = await $('[data-testid="non-existent-element"]');
        expect(await element.isExisting()).toBe(false);
      });
    });
  });

  describe('Element Interaction', () => {
    describe('Click', () => {
      it('should click a button', async () => {
        await navigateToTestPage('forms');

        const checkbox = await $('[data-testid="checkbox"]');
        expect(await checkbox.isSelected()).toBe(false);

        await checkbox.click();
        expect(await checkbox.isSelected()).toBe(true);
      });

      it('should click a link', async () => {
        const link = await $('[data-testid="nav-forms"]');
        await link.click();
        await browser.pause(100);

        const url = await browser.getUrl();
        expect(url).toContain('#forms');
      });
    });

    describe('Send Keys', () => {
      it('should type text into input', async () => {
        const input = await $('[data-testid="greet-input"]');
        await input.setValue('Hello WebDriver');

        const value = await input.getValue();
        expect(value).toBe('Hello WebDriver');
      });

      it('should clear and type new text', async () => {
        const input = await $('[data-testid="greet-input"]');
        await input.setValue('First text');
        await input.setValue('Second text');

        const value = await input.getValue();
        expect(value).toBe('Second text');
      });

      it('should type into textarea', async () => {
        await navigateToTestPage('forms');

        const textarea = await $('[data-testid="textarea"]');
        await textarea.setValue('Multi-line\ntext\ncontent');

        const value = await textarea.getValue();
        expect(value).toContain('Multi-line');
      });
    });

    describe('Clear', () => {
      it('should clear input field', async () => {
        const input = await $('[data-testid="greet-input"]');
        await input.setValue('Some text');
        await input.clearValue();

        const value = await input.getValue();
        expect(value).toBe('');
      });
    });
  });

  describe('Element State', () => {
    describe('Text Content', () => {
      it('should get element text', async () => {
        const heading = await $('[data-testid="welcome-heading"]');
        const text = await heading.getText();
        expect(text).toBe('Welcome to Tauri!');
      });

      it('should get text from paragraph', async () => {
        const paragraph = await $('[data-testid="instruction-text"]');
        const text = await paragraph.getText();
        expect(text).toContain('Click on the Tauri');
      });
    });

    describe('Tag Name', () => {
      it('should get tag name', async () => {
        const heading = await $('[data-testid="welcome-heading"]');
        const tagName = await heading.getTagName();
        expect(tagName.toLowerCase()).toBe('h1');
      });

      it('should get tag name for different elements', async () => {
        const input = await $('[data-testid="greet-input"]');
        const tagName = await input.getTagName();
        expect(tagName.toLowerCase()).toBe('input');
      });
    });

    describe('Attributes', () => {
      it('should get attribute value', async () => {
        const input = await $('[data-testid="greet-input"]');
        const placeholder = await input.getAttribute('placeholder');
        expect(placeholder).toBe('Enter a name...');
      });

      it('should get data-testid attribute', async () => {
        const heading = await $('[data-testid="welcome-heading"]');
        const testId = await heading.getAttribute('data-testid');
        expect(testId).toBe('welcome-heading');
      });

      it('should return null for non-existent attribute', async () => {
        const heading = await $('[data-testid="welcome-heading"]');
        const attr = await heading.getAttribute('non-existent');
        expect(attr).toBeNull();
      });
    });

    describe('Properties', () => {
      it('should get element property', async () => {
        const input = await $('[data-testid="greet-input"]');
        await input.setValue('test value');

        const value = await input.getProperty('value');
        expect(value).toBe('test value');
      });
    });

    describe('CSS Values', () => {
      it('should get computed CSS value', async () => {
        const heading = await $('[data-testid="welcome-heading"]');
        const display = await heading.getCSSProperty('display');
        expect(display).toBeDefined();
      });

      it('should get color CSS value', async () => {
        const element = await $('[data-testid="visible-element"]');
        const color = await element.getCSSProperty('color');
        expect(color).toBeDefined();
      });
    });

    describe('Element Rect', () => {
      it('should get element rect', async () => {
        const heading = await $('[data-testid="welcome-heading"]');
        const location = await heading.getLocation();
        const size = await heading.getSize();

        expect(location.x).toBeDefined();
        expect(location.y).toBeDefined();
        expect(size.width).toBeGreaterThan(0);
        expect(size.height).toBeGreaterThan(0);
      });

      it('should return exact size for positioned element', async () => {
        // Scroll section into view first
        const section = await $('[data-testid="rect-section"]');
        await section.scrollIntoView();

        // Element has fixed size: width: 100px, height: 80px
        const element = await $('[data-testid="positioned-element"]');
        const size = await element.getSize();

        expect(size.width).toBe(100);
        expect(size.height).toBe(80);
      });

      it('should return correct position relative to container', async () => {
        // Scroll section into view first
        const section = await $('[data-testid="rect-section"]');
        await section.scrollIntoView();

        // Element is positioned at left: 50px, top: 50px relative to container
        // Container has 1px border, so element is at container + 50 + 1 = container + 51
        const container = await $('.rect-container');
        const element = await $('[data-testid="positioned-element"]');

        const containerLocation = await container.getLocation();
        const elementLocation = await element.getLocation();

        // Element should be 51px offset from container (50px position + 1px border)
        expect(elementLocation.x).toBe(containerLocation.x + 51);
        expect(elementLocation.y).toBe(containerLocation.y + 51);
      });
    });

    describe('Displayed State', () => {
      it('should return true for visible element', async () => {
        const element = await $('[data-testid="visible-element"]');
        expect(await element.isDisplayed()).toBe(true);
      });

      it('should return false for element hidden by display:none', async () => {
        const element = await $('[data-testid="hidden-display"]');
        expect(await element.isDisplayed()).toBe(false);
      });

      it('should return false for element hidden by visibility:hidden', async () => {
        const element = await $('[data-testid="hidden-visibility"]');
        expect(await element.isDisplayed()).toBe(false);
      });
    });

    describe('Enabled State', () => {
      it('should return true for enabled button', async () => {
        const button = await $('[data-testid="enabled-button"]');
        expect(await button.isEnabled()).toBe(true);
      });

      it('should return false for disabled button', async () => {
        const button = await $('[data-testid="disabled-button"]');
        expect(await button.isEnabled()).toBe(false);
      });
    });

    describe('Selected State', () => {
      it('should return false for unchecked checkbox', async () => {
        await navigateToTestPage('forms');

        const checkbox = await $('[data-testid="checkbox"]');
        expect(await checkbox.isSelected()).toBe(false);
      });

      it('should return true for checked checkbox', async () => {
        await navigateToTestPage('forms');

        const checkbox = await $('[data-testid="checkbox"]');
        await checkbox.click();
        expect(await checkbox.isSelected()).toBe(true);
      });

      it('should handle radio button selection', async () => {
        await navigateToTestPage('forms');

        const radio1 = await $('[data-testid="radio-1"]');
        const radio2 = await $('[data-testid="radio-2"]');

        await radio1.click();
        expect(await radio1.isSelected()).toBe(true);
        expect(await radio2.isSelected()).toBe(false);

        await radio2.click();
        expect(await radio1.isSelected()).toBe(false);
        expect(await radio2.isSelected()).toBe(true);
      });
    });
  });

  describe('Accessibility', () => {
    it('should get computed ARIA role', async () => {
      const button = await $('[data-testid="greet-button"]');
      const role = await button.getComputedRole();
      expect(role).toBe('button');
    });

    it('should get computed accessible name', async () => {
      const button = await $('[data-testid="greet-button"]');
      const label = await button.getComputedLabel();
      expect(label).toBe('Greet');
    });
  });
});
