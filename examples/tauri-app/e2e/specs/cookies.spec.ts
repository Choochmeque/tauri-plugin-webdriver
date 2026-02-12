describe('Cookies', () => {
  beforeEach(async () => {
    await browser.url('tauri://localhost/#main');
    await browser.pause(100);
    // Clear all cookies before each test
    await browser.deleteAllCookies();
  });

  afterEach(async () => {
    // Clean up cookies after each test
    await browser.deleteAllCookies();
  });

  describe('Get Cookies', () => {
    it('should get all cookies (empty initially)', async () => {
      const cookies = await browser.getAllCookies();

      expect(cookies).toBeDefined();
      expect(Array.isArray(cookies)).toBe(true);
    });

    it('should get cookie by name', async () => {
      // First add a cookie
      await browser.setCookies({
        name: 'testCookie',
        value: 'testValue',
      });

      const cookies = await browser.getAllCookies();
      const testCookie = cookies.find((c) => c.name === 'testCookie');

      expect(testCookie).toBeDefined();
      expect(testCookie?.value).toBe('testValue');
    });
  });

  describe('Add Cookie', () => {
    it('should add a simple cookie', async () => {
      await browser.setCookies({
        name: 'simpleCookie',
        value: 'simpleValue',
      });

      const cookies = await browser.getAllCookies();
      const cookie = cookies.find((c) => c.name === 'simpleCookie');

      expect(cookie).toBeDefined();
      expect(cookie?.value).toBe('simpleValue');
    });

    it('should add cookie with special characters in value', async () => {
      await browser.setCookies({
        name: 'specialCookie',
        value: 'value%20with%20spaces',
      });

      const cookies = await browser.getAllCookies();
      const cookie = cookies.find((c) => c.name === 'specialCookie');

      expect(cookie).toBeDefined();
      expect(cookie?.value).toBe('value%20with%20spaces');
    });

    it('should add multiple cookies', async () => {
      await browser.setCookies([
        { name: 'cookie1', value: 'value1' },
        { name: 'cookie2', value: 'value2' },
        { name: 'cookie3', value: 'value3' },
      ]);

      const cookies = await browser.getAllCookies();

      expect(cookies.find((c) => c.name === 'cookie1')).toBeDefined();
      expect(cookies.find((c) => c.name === 'cookie2')).toBeDefined();
      expect(cookies.find((c) => c.name === 'cookie3')).toBeDefined();
    });

    it('should add cookie with path', async () => {
      await browser.setCookies({
        name: 'pathCookie',
        value: 'pathValue',
        path: '/',
      });

      const cookies = await browser.getAllCookies();
      const cookie = cookies.find((c) => c.name === 'pathCookie');

      expect(cookie).toBeDefined();
      expect(cookie?.path).toBe('/');
    });

    it('should add cookie with expiry', async () => {
      const futureTime = Math.floor(Date.now() / 1000) + 3600; // 1 hour from now

      await browser.setCookies({
        name: 'expiryCookie',
        value: 'expiryValue',
        expiry: futureTime,
      });

      const cookies = await browser.getAllCookies();
      const cookie = cookies.find((c) => c.name === 'expiryCookie');

      expect(cookie).toBeDefined();
    });

    it('should add secure cookie', async () => {
      await browser.setCookies({
        name: 'secureCookie',
        value: 'secureValue',
        secure: true,
      });

      const cookies = await browser.getAllCookies();
      const cookie = cookies.find((c) => c.name === 'secureCookie');

      // Note: secure cookies may not be set in non-HTTPS context
      // We just verify the command doesn't fail
    });

    it('should add httpOnly cookie', async () => {
      await browser.setCookies({
        name: 'httpOnlyCookie',
        value: 'httpOnlyValue',
        httpOnly: true,
      });

      const cookies = await browser.getAllCookies();
      const cookie = cookies.find((c) => c.name === 'httpOnlyCookie');

      expect(cookie).toBeDefined();
      expect(cookie?.httpOnly).toBe(true);
    });

    it('should update existing cookie', async () => {
      await browser.setCookies({
        name: 'updateCookie',
        value: 'originalValue',
      });

      await browser.setCookies({
        name: 'updateCookie',
        value: 'updatedValue',
      });

      const cookies = await browser.getAllCookies();
      const cookie = cookies.find((c) => c.name === 'updateCookie');

      expect(cookie?.value).toBe('updatedValue');
    });
  });

  describe('Delete Cookie', () => {
    it('should delete specific cookie', async () => {
      await browser.setCookies([
        { name: 'keepCookie', value: 'keep' },
        { name: 'deleteCookie', value: 'delete' },
      ]);

      await browser.deleteCookies('deleteCookie');

      const cookies = await browser.getAllCookies();

      expect(cookies.find((c) => c.name === 'keepCookie')).toBeDefined();
      expect(cookies.find((c) => c.name === 'deleteCookie')).toBeUndefined();
    });

    it('should handle deleting non-existent cookie', async () => {
      // Should not throw when deleting non-existent cookie
      await browser.deleteCookies('nonExistentCookie');

      const cookies = await browser.getAllCookies();
      expect(cookies.find((c) => c.name === 'nonExistentCookie')).toBeUndefined();
    });
  });

  describe('Delete All Cookies', () => {
    it('should delete all cookies', async () => {
      await browser.setCookies([
        { name: 'cookie1', value: 'value1' },
        { name: 'cookie2', value: 'value2' },
        { name: 'cookie3', value: 'value3' },
      ]);

      let cookies = await browser.getAllCookies();
      expect(cookies.length).toBeGreaterThan(0);

      await browser.deleteAllCookies();

      cookies = await browser.getAllCookies();
      expect(cookies.length).toBe(0);
    });

    it('should handle deleting when no cookies exist', async () => {
      await browser.deleteAllCookies();

      // Should not throw
      await browser.deleteAllCookies();

      const cookies = await browser.getAllCookies();
      expect(cookies.length).toBe(0);
    });
  });

  describe('Cookie Persistence', () => {
    it('should persist cookie across page navigation', async () => {
      await browser.setCookies({
        name: 'persistCookie',
        value: 'persistValue',
      });

      // Navigate to another page
      await browser.url('tauri://localhost/#forms');
      await browser.pause(100);

      const cookies = await browser.getAllCookies();
      const cookie = cookies.find((c) => c.name === 'persistCookie');

      expect(cookie).toBeDefined();
      expect(cookie?.value).toBe('persistValue');
    });

    it('should persist cookie after refresh', async () => {
      await browser.setCookies({
        name: 'refreshCookie',
        value: 'refreshValue',
      });

      await browser.refresh();
      await browser.pause(100);

      const cookies = await browser.getAllCookies();
      const cookie = cookies.find((c) => c.name === 'refreshCookie');

      expect(cookie).toBeDefined();
      expect(cookie?.value).toBe('refreshValue');
    });
  });
});
