// Hero terminal typing animation
// Sequences of commands typed character by character with instant output
(function () {
  const terminal = document.getElementById('hero-terminal');
  if (!terminal) return;

  // Animation sequence: each entry is either a typed command or instant output
  const sequence = [
    { type: 'cmd', text: '$ repo list' },
    { type: 'output', lines: [
      '',
      '<span class="output">REPO              BRANCH        LAST COMMIT</span>',
      '<span class="output">──────────────────────────────────────────────────────────────</span>',
      '<span class="output">api-service       </span><span class="highlight">develop</span><span class="output">       a12bc3 fix auth bug</span>',
      '<span class="output">auth-service      </span><span class="highlight">main</span><span class="output">          7a91de update deps          </span><span class="dirty">*</span>',
      '<span class="output">frontend          </span><span class="highlight">feature/ui</span><span class="output">    0ab221 add layout</span>',
      '<span class="output">worker            </span><span class="highlight">develop</span><span class="output">       c2f991 fix queue</span>',
    ]},
    { type: 'pause', ms: 1200 },
    { type: 'cmd', text: '$ repo pull' },
    { type: 'output', lines: [
      '',
      '<span class="output">── </span><span class="highlight">api-service</span><span class="output"> ──────────────────────────────</span>',
      '<span class="output">  Already up to date.</span>',
      '<span class="output">── </span><span class="highlight">auth-service</span><span class="output"> ─────────────────────────────</span>',
      '<span class="output">  Updating 7a91de..c3f210</span>',
      '',
      '  <span class="prompt">3 of 3 completed successfully.</span>',
    ]},
    { type: 'pause', ms: 1500 },
    { type: 'clear' },
    { type: 'cmd', text: '$ repo git -- log --oneline -3' },
    { type: 'output', lines: [
      '',
      '<span class="output">── </span><span class="highlight">api-service</span><span class="output"> ──────────────────────────────</span>',
      '<span class="output">  a12bc3 fix auth bug</span>',
      '<span class="output">  9f3e21 add rate limiter</span>',
      '<span class="output">  b7c403 refactor middleware</span>',
    ]},
    { type: 'pause', ms: 2000 },
    { type: 'restart' },
  ];

  let currentLine = '';

  // Type a command character by character
  function typeCommand(text, callback) {
    let i = 0;
    currentLine = '';
    const line = document.createElement('div');
    terminal.appendChild(line);

    function tick() {
      if (i < text.length) {
        currentLine += text[i];
        // Color the prompt ($) in accent, rest in primary
        const colored = currentLine.replace(/^\$/, '<span class="prompt">$</span>');
        line.innerHTML = colored + '<span class="typing-cursor"></span>';
        i++;
        setTimeout(tick, 40 + Math.random() * 40);
      } else {
        line.innerHTML = currentLine.replace(/^\$/, '<span class="prompt">$</span>');
        callback();
      }
    }
    tick();
  }

  // Show output lines with a fast cascade
  function showOutput(lines, callback) {
    let i = 0;
    function next() {
      if (i < lines.length) {
        const line = document.createElement('div');
        line.innerHTML = lines[i];
        terminal.appendChild(line);
        i++;
        setTimeout(next, 60);
      } else {
        callback();
      }
    }
    next();
  }

  // Run the full animation sequence
  function runSequence(idx) {
    if (idx >= sequence.length) return;
    const step = sequence[idx];

    switch (step.type) {
      case 'cmd':
        typeCommand(step.text, () => runSequence(idx + 1));
        break;
      case 'output':
        showOutput(step.lines, () => runSequence(idx + 1));
        break;
      case 'pause':
        setTimeout(() => runSequence(idx + 1), step.ms);
        break;
      case 'clear':
        terminal.innerHTML = '';
        runSequence(idx + 1);
        break;
      case 'restart':
        setTimeout(() => {
          terminal.innerHTML = '';
          runSequence(0);
        }, 1000);
        break;
    }
  }

  // Start after a short delay
  setTimeout(() => runSequence(0), 800);
})();
