<script>
  let scrollPosition = $state({ x: 0, y: 0 });

  function updateScrollPosition() {
    scrollPosition = {
      x: window.scrollX || document.documentElement.scrollLeft,
      y: window.scrollY || document.documentElement.scrollTop
    };
  }

  // Update scroll position on scroll
  $effect(() => {
    window.addEventListener('scroll', updateScrollPosition);
    updateScrollPosition();
    return () => window.removeEventListener('scroll', updateScrollPosition);
  });

  function scrollToBottom() {
    window.scrollTo({ top: document.body.scrollHeight, behavior: 'smooth' });
  }

  function scrollToTop() {
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  function scrollToMiddle() {
    const middle = document.getElementById('middle-marker');
    middle?.scrollIntoView({ behavior: 'smooth', block: 'center' });
  }
</script>

<section data-testid="scroll-test-page" class="scroll-page">
  <h1 data-testid="scroll-page-heading">Scroll Test Page</h1>

  <p data-testid="scroll-page-description">This page tests scrolling and viewport functionality.</p>

  <div class="scroll-controls">
    <button data-testid="scroll-to-bottom" onclick={scrollToBottom}>Scroll to Bottom</button>
    <button data-testid="scroll-to-top" onclick={scrollToTop}>Scroll to Top</button>
    <button data-testid="scroll-to-middle" onclick={scrollToMiddle}>Scroll to Middle</button>
    <p data-testid="scroll-position">Scroll position: X={scrollPosition.x}, Y={scrollPosition.y}</p>
  </div>

  <div data-testid="top-marker" class="marker top-marker">
    <h2>Top of Page</h2>
    <p>This is the top marker element.</p>
  </div>

  <!-- Generate some scrollable content -->
  {#each Array(20) as _, i}
    <div class="content-block" data-testid="content-block-{i}">
      <h3>Content Block {i + 1}</h3>
      <p>This is filler content to create a scrollable page. Block number {i + 1} of 20.</p>
      <p>Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>
    </div>
  {/each}

  <div id="middle-marker" data-testid="middle-marker" class="marker middle-marker">
    <h2>Middle of Page</h2>
    <p>This is the middle marker element.</p>
    <button data-testid="middle-button">Middle Button</button>
  </div>

  {#each Array(20) as _, i}
    <div class="content-block" data-testid="content-block-{i + 20}">
      <h3>Content Block {i + 21}</h3>
      <p>More filler content. Block number {i + 21} of 40.</p>
      <p>Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.</p>
    </div>
  {/each}

  <div data-testid="bottom-marker" class="marker bottom-marker">
    <h2>Bottom of Page</h2>
    <p>This is the bottom marker element.</p>
    <button data-testid="bottom-button">Bottom Button</button>
    <input data-testid="bottom-input" placeholder="Bottom input..." />
  </div>
</section>

<style>
  .scroll-page {
    min-height: 3000px;
  }

  .scroll-controls {
    position: fixed;
    top: 60px;
    right: 10px;
    background: rgba(0, 0, 0, 0.8);
    padding: 1rem;
    border-radius: 4px;
    z-index: 100;
  }

  .scroll-controls button {
    display: block;
    margin-bottom: 0.5rem;
    width: 100%;
  }

  .marker {
    padding: 2rem;
    margin: 2rem 0;
    border: 2px solid;
    border-radius: 8px;
    text-align: center;
  }

  .top-marker {
    border-color: #4caf50;
    background: rgba(76, 175, 80, 0.2);
  }

  .middle-marker {
    border-color: #ff9800;
    background: rgba(255, 152, 0, 0.2);
  }

  .bottom-marker {
    border-color: #f44336;
    background: rgba(244, 67, 54, 0.2);
  }

  .content-block {
    padding: 1rem;
    margin: 1rem 0;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 4px;
  }

  .content-block h3 {
    margin-top: 0;
  }
</style>
