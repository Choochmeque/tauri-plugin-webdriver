describe('Script Execution', () => {
  beforeEach(async () => {
    await browser.url('tauri://localhost/#main');
    await browser.pause(100);
  });

  describe('Synchronous Script Execution', () => {
    it('should execute simple script', async () => {
      const result = await browser.execute(() => {
        return 1 + 1;
      });
      expect(result).toBe(2);
    });

    it('should execute script returning string', async () => {
      const result = await browser.execute(() => {
        return 'Hello from script';
      });
      expect(result).toBe('Hello from script');
    });

    it('should execute script returning object', async () => {
      const result = await browser.execute(() => {
        return { name: 'test', value: 42 };
      });
      expect(result).toEqual({ name: 'test', value: 42 });
    });

    it('should execute script returning array', async () => {
      const result = await browser.execute(() => {
        return [1, 2, 3, 4, 5];
      });
      expect(result).toEqual([1, 2, 3, 4, 5]);
    });

    it('should execute script returning null', async () => {
      const result = await browser.execute(() => {
        return null;
      });
      expect(result).toBeNull();
    });

    it('should execute script returning undefined', async () => {
      const result = await browser.execute(() => {
        return undefined;
      });
      // JSON serialization converts undefined to null (WebDriver spec behavior)
      expect(result).toBeNull();
    });

    it('should execute script returning boolean', async () => {
      const trueResult = await browser.execute(() => true);
      const falseResult = await browser.execute(() => false);

      expect(trueResult).toBe(true);
      expect(falseResult).toBe(false);
    });

    it('should access DOM in script', async () => {
      const result = await browser.execute(() => {
        return document.title;
      });
      expect(result).toBe('Tauri + Svelte');
    });

    it('should find elements in script', async () => {
      const result = await browser.execute(() => {
        const heading = document.querySelector('[data-testid="welcome-heading"]');
        return heading ? heading.textContent : null;
      });
      expect(result).toBe('Welcome to Tauri!');
    });

    it('should modify DOM in script', async () => {
      await browser.execute(() => {
        const heading = document.querySelector('[data-testid="welcome-heading"]');
        if (heading) {
          heading.textContent = 'Modified Heading';
        }
      });

      const heading = await $('[data-testid="welcome-heading"]');
      const text = await heading.getText();
      expect(text).toBe('Modified Heading');
    });
  });

  describe('Script with Arguments', () => {
    it('should pass single argument', async () => {
      const result = await browser.execute((num) => {
        return num * 2;
      }, 21);
      expect(result).toBe(42);
    });

    it('should pass multiple arguments', async () => {
      const result = await browser.execute((a, b, c) => {
        return a + b + c;
      }, 1, 2, 3);
      expect(result).toBe(6);
    });

    it('should pass string argument', async () => {
      const result = await browser.execute((str) => {
        return str.toUpperCase();
      }, 'hello');
      expect(result).toBe('HELLO');
    });

    it('should pass object argument', async () => {
      const result = await browser.execute((obj) => {
        return obj.value * 2;
      }, { value: 10 });
      expect(result).toBe(20);
    });

    it('should pass array argument', async () => {
      const result = await browser.execute((arr) => {
        return arr.reduce((sum, n) => sum + n, 0);
      }, [1, 2, 3, 4, 5]);
      expect(result).toBe(15);
    });

    it('should pass element as argument', async () => {
      const input = await $('[data-testid="greet-input"]');
      await input.setValue('test value');

      const result = await browser.execute((el) => {
        return el.value;
      }, input);
      expect(result).toBe('test value');
    });
  });

  describe('Asynchronous Script Execution', () => {
    it('should execute async script with immediate callback', async () => {
      const result = await browser.executeAsync((done) => {
        done('async result');
      });
      expect(result).toBe('async result');
    });

    it('should execute async script with setTimeout', async () => {
      const result = await browser.executeAsync((done) => {
        setTimeout(() => {
          done('delayed result');
        }, 100);
      });
      expect(result).toBe('delayed result');
    });

    it('should execute async script with arguments', async () => {
      const result = await browser.executeAsync((multiplier, done) => {
        setTimeout(() => {
          done(multiplier * 10);
        }, 50);
      }, 5);
      expect(result).toBe(50);
    });

    it('should execute async script returning object', async () => {
      const result = await browser.executeAsync((done) => {
        setTimeout(() => {
          done({ status: 'complete', count: 42 });
        }, 50);
      });
      expect(result).toEqual({ status: 'complete', count: 42 });
    });

    it('should execute async script with Promise', async () => {
      const result = await browser.executeAsync((done) => {
        new Promise((resolve) => {
          setTimeout(() => resolve('promise result'), 50);
        }).then(done);
      });
      expect(result).toBe('promise result');
    });

    it('should execute async script with fetch simulation', async () => {
      const result = await browser.executeAsync((done) => {
        // Simulate async operation
        setTimeout(() => {
          done({ data: [1, 2, 3], success: true });
        }, 100);
      });
      expect(result.success).toBe(true);
      expect(result.data).toEqual([1, 2, 3]);
    });
  });

  describe('Script Error Handling', () => {
    it('should handle script that throws error', async () => {
      let errorThrown = false;
      try {
        await browser.execute(() => {
          throw new Error('Intentional error');
        });
      } catch (e) {
        errorThrown = true;
      }
      expect(errorThrown).toBe(true);
    });

    it('should handle script with syntax error gracefully', async () => {
      // This should throw during execution
      let errorThrown = false;
      try {
        await browser.execute('this is not valid javascript {{{');
      } catch (e) {
        errorThrown = true;
      }
      expect(errorThrown).toBe(true);
    });
  });

  describe('Complex Script Operations', () => {
    it('should create and return complex nested object', async () => {
      const result = await browser.execute(() => {
        return {
          level1: {
            level2: {
              level3: {
                value: 'deep',
              },
            },
          },
          array: [1, { nested: true }, [2, 3]],
        };
      });
      expect(result.level1.level2.level3.value).toBe('deep');
      expect(result.array[1].nested).toBe(true);
    });

    it('should get window dimensions', async () => {
      const result = await browser.execute(() => {
        return {
          innerWidth: window.innerWidth,
          innerHeight: window.innerHeight,
          outerWidth: window.outerWidth,
          outerHeight: window.outerHeight,
        };
      });
      expect(result.innerWidth).toBeGreaterThan(0);
      expect(result.innerHeight).toBeGreaterThan(0);
    });

    it('should get element count', async () => {
      const result = await browser.execute(() => {
        return document.querySelectorAll('[data-testid]').length;
      });
      expect(result).toBeGreaterThan(0);
    });
  });
});
