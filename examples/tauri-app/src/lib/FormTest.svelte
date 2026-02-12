<script>
  let formData = $state({
    textInput: '',
    passwordInput: '',
    emailInput: '',
    numberInput: 0,
    textarea: '',
    checkbox: false,
    radioValue: '',
    selectValue: '',
    multiSelectValues: [],
    rangeValue: 50,
    dateValue: '',
    colorValue: '#000000',
    fileValue: null,
  });

  let formSubmitted = $state(false);
  let submitData = $state(null);

  function handleSubmit(event) {
    event.preventDefault();
    formSubmitted = true;
    submitData = { ...formData };
  }

  function resetForm() {
    formData = {
      textInput: '',
      passwordInput: '',
      emailInput: '',
      numberInput: 0,
      textarea: '',
      checkbox: false,
      radioValue: '',
      selectValue: '',
      multiSelectValues: [],
      rangeValue: 50,
      dateValue: '',
      colorValue: '#000000',
      fileValue: null,
    };
    formSubmitted = false;
    submitData = null;
  }
</script>

<section data-testid="form-test-page">
  <h1 data-testid="form-page-heading">Form Test Page</h1>

  <p data-testid="form-page-description">This page tests form elements and interactions.</p>

  <form data-testid="test-form" onsubmit={handleSubmit}>
    <!-- Text Inputs -->
    <fieldset class="form-section">
      <legend>Text Inputs</legend>

      <div class="form-group">
        <label for="text-input">Text Input:</label>
        <input
          type="text"
          id="text-input"
          data-testid="text-input"
          placeholder="Enter text..."
          bind:value={formData.textInput}
        />
      </div>

      <div class="form-group">
        <label for="password-input">Password Input:</label>
        <input
          type="password"
          id="password-input"
          data-testid="password-input"
          placeholder="Enter password..."
          bind:value={formData.passwordInput}
        />
      </div>

      <div class="form-group">
        <label for="email-input">Email Input:</label>
        <input
          type="email"
          id="email-input"
          data-testid="email-input"
          placeholder="email@example.com"
          bind:value={formData.emailInput}
        />
      </div>

      <div class="form-group">
        <label for="number-input">Number Input:</label>
        <input
          type="number"
          id="number-input"
          data-testid="number-input"
          min="0"
          max="100"
          bind:value={formData.numberInput}
        />
      </div>

      <div class="form-group">
        <label for="textarea">Textarea:</label>
        <textarea
          id="textarea"
          data-testid="textarea"
          placeholder="Enter multi-line text..."
          rows="4"
          bind:value={formData.textarea}
        ></textarea>
      </div>

      <div class="form-group">
        <label for="readonly-input">Readonly Input:</label>
        <input
          type="text"
          id="readonly-input"
          data-testid="readonly-input"
          value="This is readonly"
          readonly
        />
      </div>

      <div class="form-group">
        <label for="disabled-input">Disabled Input:</label>
        <input
          type="text"
          id="disabled-input"
          data-testid="disabled-input"
          value="This is disabled"
          disabled
        />
      </div>
    </fieldset>

    <!-- Checkboxes and Radio Buttons -->
    <fieldset class="form-section">
      <legend>Checkboxes & Radio Buttons</legend>

      <div class="form-group">
        <label>
          <input
            type="checkbox"
            id="checkbox"
            data-testid="checkbox"
            bind:checked={formData.checkbox}
          />
          Single Checkbox
        </label>
      </div>

      <div class="form-group">
        <span>Checkbox Group:</span>
        <label>
          <input type="checkbox" data-testid="checkbox-1" value="option1" />
          Option 1
        </label>
        <label>
          <input type="checkbox" data-testid="checkbox-2" value="option2" />
          Option 2
        </label>
        <label>
          <input type="checkbox" data-testid="checkbox-3" value="option3" />
          Option 3
        </label>
      </div>

      <div class="form-group">
        <span>Radio Buttons:</span>
        <label>
          <input
            type="radio"
            name="radio-group"
            data-testid="radio-1"
            value="radio1"
            bind:group={formData.radioValue}
          />
          Radio 1
        </label>
        <label>
          <input
            type="radio"
            name="radio-group"
            data-testid="radio-2"
            value="radio2"
            bind:group={formData.radioValue}
          />
          Radio 2
        </label>
        <label>
          <input
            type="radio"
            name="radio-group"
            data-testid="radio-3"
            value="radio3"
            bind:group={formData.radioValue}
          />
          Radio 3
        </label>
      </div>
    </fieldset>

    <!-- Select Elements -->
    <fieldset class="form-section">
      <legend>Select Elements</legend>

      <div class="form-group">
        <label for="select">Single Select:</label>
        <select
          id="select"
          data-testid="select"
          bind:value={formData.selectValue}
        >
          <option value="">-- Select an option --</option>
          <option value="opt1">Option 1</option>
          <option value="opt2">Option 2</option>
          <option value="opt3">Option 3</option>
          <option value="opt4" disabled>Option 4 (disabled)</option>
        </select>
      </div>

      <div class="form-group">
        <label for="multi-select">Multiple Select:</label>
        <select
          id="multi-select"
          data-testid="multi-select"
          multiple
          bind:value={formData.multiSelectValues}
        >
          <option value="multi1">Multi Option 1</option>
          <option value="multi2">Multi Option 2</option>
          <option value="multi3">Multi Option 3</option>
          <option value="multi4">Multi Option 4</option>
        </select>
      </div>
    </fieldset>

    <!-- Other Input Types -->
    <fieldset class="form-section">
      <legend>Other Input Types</legend>

      <div class="form-group">
        <label for="range-input">Range Input: {formData.rangeValue}</label>
        <input
          type="range"
          id="range-input"
          data-testid="range-input"
          min="0"
          max="100"
          bind:value={formData.rangeValue}
        />
      </div>

      <div class="form-group">
        <label for="date-input">Date Input:</label>
        <input
          type="date"
          id="date-input"
          data-testid="date-input"
          bind:value={formData.dateValue}
        />
      </div>

      <div class="form-group">
        <label for="color-input">Color Input:</label>
        <input
          type="color"
          id="color-input"
          data-testid="color-input"
          bind:value={formData.colorValue}
        />
      </div>

      <div class="form-group">
        <label for="file-input">File Input:</label>
        <input
          type="file"
          id="file-input"
          data-testid="file-input"
        />
      </div>
    </fieldset>

    <!-- Form Actions -->
    <div class="form-actions">
      <button type="submit" data-testid="submit-button">Submit</button>
      <button type="button" data-testid="reset-button" onclick={resetForm}>Reset</button>
      <button type="button" data-testid="disabled-submit" disabled>Disabled Button</button>
    </div>
  </form>

  <!-- Form Result -->
  {#if formSubmitted}
    <div data-testid="form-result" class="form-result">
      <h2>Form Submitted!</h2>
      <pre data-testid="form-data">{JSON.stringify(submitData, null, 2)}</pre>
    </div>
  {/if}

  <!-- Display current form values -->
  <div data-testid="current-values" class="current-values">
    <h2>Current Form Values:</h2>
    <p data-testid="text-value">Text: {formData.textInput}</p>
    <p data-testid="checkbox-value">Checkbox: {formData.checkbox}</p>
    <p data-testid="radio-value">Radio: {formData.radioValue}</p>
    <p data-testid="select-value">Select: {formData.selectValue}</p>
  </div>
</section>

<style>
  .form-section {
    margin: 1rem 0;
    padding: 1rem;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
  }

  .form-section legend {
    padding: 0 0.5rem;
    font-weight: bold;
  }

  .form-group {
    margin: 0.75rem 0;
  }

  .form-group label {
    display: inline-block;
    margin-right: 1rem;
  }

  .form-group input[type="text"],
  .form-group input[type="password"],
  .form-group input[type="email"],
  .form-group input[type="number"],
  .form-group input[type="date"],
  .form-group textarea,
  .form-group select {
    display: block;
    width: 100%;
    max-width: 300px;
    padding: 0.5rem;
    margin-top: 0.25rem;
  }

  .form-group input[type="range"] {
    width: 200px;
  }

  .form-actions {
    margin-top: 1rem;
    padding: 1rem;
    border-top: 1px solid rgba(255, 255, 255, 0.2);
  }

  .form-actions button {
    margin-right: 0.5rem;
    padding: 0.5rem 1rem;
  }

  .form-result {
    margin-top: 1rem;
    padding: 1rem;
    background: rgba(76, 175, 80, 0.2);
    border: 1px solid #4caf50;
    border-radius: 4px;
  }

  .form-result pre {
    background: rgba(0, 0, 0, 0.3);
    padding: 1rem;
    overflow: auto;
    font-size: 0.8rem;
  }

  .current-values {
    margin-top: 1rem;
    padding: 1rem;
    background: rgba(100, 149, 237, 0.2);
    border: 1px solid cornflowerblue;
    border-radius: 4px;
  }

  .current-values h2 {
    margin-top: 0;
    font-size: 1rem;
  }
</style>
