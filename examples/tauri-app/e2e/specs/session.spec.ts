describe('Session Management', () => {
  describe('GET /status', () => {
    it('should return server status', async () => {
      const status = await browser.status();
      expect(status).toBeDefined();
      expect(status.ready).toBe(true);
    });
  });

  describe('Session lifecycle', () => {
    it('should have an active session', async () => {
      // WebdriverIO automatically creates a session
      const sessionId = browser.sessionId;
      expect(sessionId).toBeDefined();
      expect(typeof sessionId).toBe('string');
      expect(sessionId.length).toBeGreaterThan(0);
    });

    it('should be able to get session capabilities', async () => {
      const capabilities = browser.capabilities;
      expect(capabilities).toBeDefined();
    });
  });

  describe('Timeouts', () => {
    it('should get current timeouts', async () => {
      const timeouts = await browser.getTimeouts();
      expect(timeouts).toBeDefined();
      expect(typeof timeouts.implicit).toBe('number');
      expect(typeof timeouts.pageLoad).toBe('number');
      expect(typeof timeouts.script).toBe('number');
    });

    it('should set implicit timeout', async () => {
      await browser.setTimeout({ implicit: 5000 });
      const timeouts = await browser.getTimeouts();
      expect(timeouts.implicit).toBe(5000);

      // Reset to default
      await browser.setTimeout({ implicit: 0 });
    });

    it('should set page load timeout', async () => {
      await browser.setTimeout({ pageLoad: 60000 });
      const timeouts = await browser.getTimeouts();
      expect(timeouts.pageLoad).toBe(60000);

      // Reset to default
      await browser.setTimeout({ pageLoad: 300000 });
    });

    it('should set script timeout', async () => {
      await browser.setTimeout({ script: 10000 });
      const timeouts = await browser.getTimeouts();
      expect(timeouts.script).toBe(10000);

      // Reset to default
      await browser.setTimeout({ script: 30000 });
    });

    it('should set multiple timeouts at once', async () => {
      await browser.setTimeout({
        implicit: 1000,
        pageLoad: 30000,
        script: 15000,
      });

      const timeouts = await browser.getTimeouts();
      expect(timeouts.implicit).toBe(1000);
      expect(timeouts.pageLoad).toBe(30000);
      expect(timeouts.script).toBe(15000);

      // Reset to defaults
      await browser.setTimeout({
        implicit: 0,
        pageLoad: 300000,
        script: 30000,
      });
    });
  });
});
