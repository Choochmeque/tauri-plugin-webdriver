describe('Frames', () => {
  beforeEach(async () => {
    await browser.url('tauri://localhost/#frames');
    await browser.pause(100);
  });

  describe('Switch to Frame', () => {
    it('should switch to frame by index', async () => {
      // Switch to first frame (index 0)
      await browser.switchToFrame(0);

      // Verify we're in the frame by finding frame-specific element
      const frameContent = await $('[data-testid="frame-content"]');
      expect(await frameContent.isExisting()).toBe(true);

      const text = await frameContent.getText();
      expect(text).toBe('Inside Frame');

      // Switch back to top
      await browser.switchToFrame(null);
    });

    it('should switch to frame by element', async () => {
      const frameElement = await $('[data-testid="test-frame"]');

      await browser.switchToFrame(frameElement);

      // Verify we're in the frame
      const frameContent = await $('[data-testid="frame-content"]');
      expect(await frameContent.isExisting()).toBe(true);

      await browser.switchToFrame(null);
    });

    it('should switch to second frame', async () => {
      // Switch to second frame (index 1)
      await browser.switchToFrame(1);

      // Verify we're in the second frame
      const frameContent = await $('[data-testid="frame-content"]');
      expect(await frameContent.isExisting()).toBe(true);

      const text = await frameContent.getText();
      expect(text).toBe('Inside Frame 2');

      await browser.switchToFrame(null);
    });

    it('should switch to frame by element reference', async () => {
      const frame2 = await $('[data-testid="test-frame-2"]');

      await browser.switchToFrame(frame2);

      const frameContent = await $('[data-testid="frame-content"]');
      const text = await frameContent.getText();
      expect(text).toBe('Inside Frame 2');

      await browser.switchToFrame(null);
    });
  });

  describe('Switch to Top Frame', () => {
    it('should switch back to top frame with null', async () => {
      // First switch into a frame
      await browser.switchToFrame(0);

      // Verify we're in frame
      let frameContent = await $('[data-testid="frame-content"]');
      expect(await frameContent.isExisting()).toBe(true);

      // Switch back to top
      await browser.switchToFrame(null);

      // Verify we're back in main content
      const pageHeading = await $('[data-testid="frame-page-heading"]');
      expect(await pageHeading.isExisting()).toBe(true);
    });

    it('should not find main page elements when in frame', async () => {
      await browser.switchToFrame(0);

      // Main page heading should not exist in frame context
      const pageHeading = await $('[data-testid="frame-page-heading"]');
      expect(await pageHeading.isExisting()).toBe(false);

      await browser.switchToFrame(null);
    });
  });

  describe('Switch to Parent Frame', () => {
    it('should switch to parent frame', async () => {
      // Switch into frame
      await browser.switchToFrame(0);

      // Verify we're in frame
      const frameContent = await $('[data-testid="frame-content"]');
      expect(await frameContent.isExisting()).toBe(true);

      // Switch to parent
      await browser.switchToParentFrame();

      // Should be back in main content
      const pageHeading = await $('[data-testid="frame-page-heading"]');
      expect(await pageHeading.isExisting()).toBe(true);
    });
  });

  describe('Interact with Frame Content', () => {
    it('should click button inside frame', async () => {
      await browser.switchToFrame(0);

      const frameButton = await $('[data-testid="frame-button"]');
      await frameButton.click();

      // Button exists and was clickable
      expect(await frameButton.isExisting()).toBe(true);

      await browser.switchToFrame(null);
    });

    it('should type in input inside frame', async () => {
      await browser.switchToFrame(0);

      const frameInput = await $('[data-testid="frame-input"]');
      await frameInput.setValue('Frame input text');

      const value = await frameInput.getValue();
      expect(value).toBe('Frame input text');

      await browser.switchToFrame(null);
    });

    it('should get text from frame element', async () => {
      await browser.switchToFrame(0);

      const frameText = await $('[data-testid="frame-text"]');
      const text = await frameText.getText();
      expect(text).toBe('This is content inside the iframe.');

      await browser.switchToFrame(null);
    });
  });

  describe('Frame Switching Between Frames', () => {
    it('should switch between frames', async () => {
      // Switch to first frame
      await browser.switchToFrame(0);
      let content = await $('[data-testid="frame-content"]');
      expect(await content.getText()).toBe('Inside Frame');

      // Switch back to top
      await browser.switchToFrame(null);

      // Switch to second frame
      await browser.switchToFrame(1);
      content = await $('[data-testid="frame-content"]');
      expect(await content.getText()).toBe('Inside Frame 2');

      await browser.switchToFrame(null);
    });

    it('should handle rapid frame switching', async () => {
      for (let i = 0; i < 3; i++) {
        await browser.switchToFrame(0);
        const content = await $('[data-testid="frame-content"]');
        expect(await content.isExisting()).toBe(true);

        await browser.switchToFrame(null);
        const heading = await $('[data-testid="frame-page-heading"]');
        expect(await heading.isExisting()).toBe(true);
      }
    });
  });

  describe('Outside Frame Content', () => {
    it('should interact with content outside frames', async () => {
      const outsideButton = await $('[data-testid="outside-button"]');
      expect(await outsideButton.isExisting()).toBe(true);

      await outsideButton.click();
    });

    it('should verify outside content not accessible from frame', async () => {
      await browser.switchToFrame(0);

      const outsideButton = await $('[data-testid="outside-button"]');
      expect(await outsideButton.isExisting()).toBe(false);

      await browser.switchToFrame(null);
    });
  });

  describe('Frame Error Handling', () => {
    it('should handle invalid frame index', async () => {
      let errorThrown = false;
      try {
        await browser.switchToFrame(99);
      } catch (e) {
        errorThrown = true;
      }
      expect(errorThrown).toBe(true);
    });
  });
});
