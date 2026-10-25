// Vectrust Documentation App Logic
document.addEventListener('DOMContentLoaded', () => {
  // 1. Initialize Mermaid
  if (window.mermaid) {
    mermaid.initialize({
      startOnLoad: true,
      theme: 'dark',
      themeVariables: {
        darkMode: true,
        primaryColor: '#6366f1',
        primaryBorderColor: '#818cf8',
        lineColor: '#94a3b8',
        fontFamily: 'Inter, sans-serif'
      }
    });
  }

  // 2. Theme Toggle
  const themeToggle = document.getElementById('theme-toggle');
  const themeIcon = themeToggle.querySelector('.theme-icon');
  const htmlEl = document.documentElement;

  const savedTheme = localStorage.getItem('vectrust_theme') || 'dark';
  htmlEl.setAttribute('data-theme', savedTheme);
  themeIcon.textContent = savedTheme === 'dark' ? '🌙' : '☀️';

  themeToggle.addEventListener('click', () => {
    const current = htmlEl.getAttribute('data-theme');
    const next = current === 'dark' ? 'light' : 'dark';
    htmlEl.setAttribute('data-theme', next);
    localStorage.setItem('vectrust_theme', next);
    themeIcon.textContent = next === 'dark' ? '🌙' : '☀️';
  });

  // 3. Mobile Menu Toggle
  const menuToggle = document.getElementById('menu-toggle');
  const sidebar = document.getElementById('sidebar');
  if (menuToggle && sidebar) {
    menuToggle.addEventListener('click', () => {
      sidebar.classList.toggle('open');
    });

    // Close when clicking sidebar item on mobile
    sidebar.querySelectorAll('.sidebar-item').forEach(item => {
      item.addEventListener('click', () => {
        if (window.innerWidth <= 900) {
          sidebar.classList.remove('open');
        }
      });
    });
  }

  // 4. Copy to Clipboard
  document.querySelectorAll('.copy-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      const targetId = btn.getAttribute('data-clipboard-target');
      const targetEl = document.querySelector(targetId);
      if (targetEl) {
        try {
          await navigator.clipboard.writeText(targetEl.textContent);
          const original = btn.textContent;
          btn.textContent = 'Copied!';
          btn.style.borderColor = 'var(--color-success)';
          btn.style.color = 'var(--color-success)';
          setTimeout(() => {
            btn.textContent = original;
            btn.style.borderColor = '';
            btn.style.color = '';
          }, 2000);
        } catch (err) {
          console.error('Clipboard copy failed:', err);
        }
      }
    });
  });

  // 5. ScrollSpy for Sidebar Items
  const sections = document.querySelectorAll('section.section');
  const sidebarLinks = document.querySelectorAll('.sidebar-item');

  function updateActiveLink() {
    let currentId = '';
    const scrollPos = window.scrollY + 120;

    sections.forEach(sec => {
      if (sec.offsetTop <= scrollPos) {
        currentId = sec.getAttribute('id');
      }
    });

    sidebarLinks.forEach(link => {
      const href = link.getAttribute('href');
      if (href === `#${currentId}`) {
        link.classList.add('active');
      } else {
        link.classList.remove('active');
      }
    });
  }
  window.addEventListener('scroll', updateActiveLink);
  updateActiveLink();

  // 6. Search Modal Engine
  const searchTrigger = document.getElementById('search-trigger');
  const searchModal = document.getElementById('search-modal');
  const modalCloseEsc = document.getElementById('modal-close-esc');
  const modalInput = document.getElementById('modal-search-input');
  const modalResults = document.getElementById('modal-search-results');

  const searchIndex = [
    { title: 'Overview & Highlights', snippet: 'Embedded-first, pure Rust, HNSW, SIMD distance, WAL crash recovery', link: '#overview' },
    { title: 'Quickstart Guide', snippet: 'Adding dependencies to Cargo.toml, creating collection, inserting and searching', link: '#quickstart' },
    { title: 'Vectrust CLI Tool', snippet: 'vectrust info, check, benchmark, repl commands, pre-compiled binaries', link: '#cli' },
    { title: 'Architecture & Internals', snippet: 'Collection concurrency, RwLock without poisoning, Write-Ahead Log, Snapshots', link: '#architecture' },
    { title: 'HNSW Graph Mechanics', snippet: 'Proximity graph parameters: m, m0, ef_construction, ef_search beam size', link: '#hnsw' },
    { title: 'SIMD Vector Distance Metrics', snippet: 'Cosine similarity, Euclidean (L2), Dot Product (MIPS), Manhattan (L1)', link: '#simd' },
    { title: 'Metadata Filtering', snippet: 'Structured JSON boolean expressions: Eq, In, Gt, Lt, And, Or, Not, Contains', link: '#filtering' },
    { title: 'WAL & Atomic Snapshots', snippet: 'Crash resilience, CRC32 checksum verification, auto-compaction', link: '#persistence' },
    { title: 'Performance Benchmarks', snippet: 'Sub-millisecond latency, 24,000+ QPS, 99.4% recall, comparison vs Flat', link: '#benchmarks' },
    { title: 'Concurrency & Thread Safety', snippet: 'Multi-threaded Rayon batch search and concurrent readers', link: '#concurrency' },
    { title: 'Interactive Vector Playground', snippet: 'Simulate 2D vector cosine & euclidean search in browser canvas', link: '#playground' },
    { title: 'Vectrust::open(path)', snippet: 'Opens or creates persistent embedded database engine on disk', link: '#api-vectrust' },
    { title: 'Vectrust::in_memory()', snippet: 'Creates ephemeral in-memory database instance', link: '#api-vectrust' },
    { title: 'Collection::search(query)', snippet: 'Executes nearest neighbor vector search with optional filter', link: '#api-collection' },
    { title: 'SearchQuery::new(vector, k)', snippet: 'Configure top-k, exploration depth ef_search, and payload inclusion', link: '#api-types' }
  ];

  function openSearch() {
    searchModal.classList.add('open');
    modalInput.value = '';
    modalInput.focus();
    renderSearchResults('');
  }

  function closeSearch() {
    searchModal.classList.remove('open');
  }

  if (searchTrigger) searchTrigger.addEventListener('click', openSearch);
  if (modalCloseEsc) modalCloseEsc.addEventListener('click', closeSearch);

  searchModal.addEventListener('click', (e) => {
    if (e.target === searchModal) closeSearch();
  });

  window.addEventListener('keydown', (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      openSearch();
    }
    if (e.key === 'Escape' && searchModal.classList.contains('open')) {
      closeSearch();
    }
  });

  function renderSearchResults(query) {
    const q = query.trim().toLowerCase();
    if (!q) {
      modalResults.innerHTML = '<div class="search-empty-hint">Type to start searching...</div>';
      return;
    }

    const matches = searchIndex.filter(item =>
      item.title.toLowerCase().includes(q) || item.snippet.toLowerCase().includes(q)
    );

    if (matches.length === 0) {
      modalResults.innerHTML = '<div class="search-empty-hint">No documentation results found.</div>';
      return;
    }

    modalResults.innerHTML = matches.map(m => `
      <a href="${m.link}" class="search-result-item" onclick="document.getElementById('search-modal').classList.remove('open')">
        <div class="search-result-title">${m.title}</div>
        <div class="search-result-snippet">${m.snippet}</div>
      </a>
    `).join('');
  }

  modalInput.addEventListener('input', (e) => {
    renderSearchResults(e.target.value);
  });

  // 7. Interactive 2D Vector Playground Simulator
  initVectorPlayground();
});

function initVectorPlayground() {
  const canvas = document.getElementById('vector-canvas');
  if (!canvas) return;
  const ctx = canvas.getContext('2d');

  let points = [
    { id: 101, x: 180, y: 120, label: 'Doc 101: Rust Concurrency', category: 'systems' },
    { id: 102, x: 220, y: 140, label: 'Doc 102: HNSW Graphs', category: 'ai' },
    { id: 103, x: 450, y: 310, label: 'Doc 103: Cloud Kubernetes', category: 'cloud' },
    { id: 104, x: 280, y: 170, label: 'Doc 104: Vector Embeddings', category: 'ai' },
    { id: 105, x: 520, y: 260, label: 'Doc 105: Postgres WAL Logs', category: 'systems' },
    { id: 106, x: 420, y: 360, label: 'Doc 106: Serverless Architecture', category: 'cloud' },
    { id: 107, x: 160, y: 230, label: 'Doc 107: SIMD AVX2 Intrinsics', category: 'systems' },
    { id: 108, x: 310, y: 220, label: 'Doc 108: Transformers & RAG', category: 'ai' }
  ];

  let query = { x: 240, y: 160 };
  let isDragging = false;

  const metricSelect = document.getElementById('metric-select');
  const kSlider = document.getElementById('k-slider');
  const kVal = document.getElementById('k-val');
  const filterTag = document.getElementById('filter-tag');
  const btnAddRandom = document.getElementById('btn-add-random');
  const btnReset = document.getElementById('btn-reset-demo');
  const resultsContainer = document.getElementById('results-table-container');

  function calculateDistance(p1, p2, metric) {
    const dx = p1.x - p2.x;
    const dy = p1.y - p2.y;

    if (metric === 'euclidean') {
      return Math.sqrt(dx * dx + dy * dy);
    } else if (metric === 'manhattan') {
      return Math.abs(dx) + Math.abs(dy);
    } else {
      // Cosine similarity in 2D centered from origin (350, 225)
      const cx = canvas.width / 2;
      const cy = canvas.height / 2;
      const v1x = p1.x - cx;
      const v1y = p1.y - cy;
      const v2x = p2.x - cx;
      const v2y = p2.y - cy;

      const dot = v1x * v2x + v1y * v2y;
      const mag1 = Math.sqrt(v1x * v1x + v1y * v1y);
      const mag2 = Math.sqrt(v2x * v2x + v2y * v2y);

      if (mag1 === 0 || mag2 === 0) return 0;
      const cosSim = dot / (mag1 * mag2);
      // Return 1 - cosSim as distance
      return 1.0 - cosSim;
    }
  }

  function runSearch() {
    const metric = metricSelect.value;
    const k = parseInt(kSlider.value, 10);
    const category = filterTag.value;

    let eligible = points.filter(p => category === 'all' || p.category === category);

    let scored = eligible.map(p => {
      const dist = calculateDistance(p, query, metric);
      let score;
      if (metric === 'cosine') {
        score = (1.0 - dist).toFixed(4); // similarity
      } else {
        score = (1.0 / (1.0 + dist * 0.02)).toFixed(4);
      }
      return { ...p, distance: dist, score: parseFloat(score) };
    });

    scored.sort((a, b) => a.distance - b.distance);
    const topK = scored.slice(0, k);
    const topKIds = new Set(topK.map(p => p.id));

    draw(topKIds);
    renderTable(topK, metric);
  }

  function draw(topKIds) {
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    // Draw grid
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.05)';
    ctx.lineWidth = 1;
    for (let x = 0; x < canvas.width; x += 40) {
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, canvas.height);
      ctx.stroke();
    }
    for (let y = 0; y < canvas.height; y += 40) {
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(canvas.width, y);
      ctx.stroke();
    }

    // Origin center
    const cx = canvas.width / 2;
    const cy = canvas.height / 2;
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.12)';
    ctx.setLineDash([4, 4]);
    ctx.beginPath();
    ctx.moveTo(cx, 0); ctx.lineTo(cx, canvas.height);
    ctx.moveTo(0, cy); ctx.lineTo(canvas.width, cy);
    ctx.stroke();
    ctx.setLineDash([]);

    // Draw connection lines to top-K matches
    points.forEach(p => {
      if (topKIds.has(p.id)) {
        ctx.strokeStyle = 'rgba(16, 185, 129, 0.5)';
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(query.x, query.y);
        ctx.lineTo(p.x, p.y);
        ctx.stroke();
      }
    });

    // Draw indexed points
    points.forEach(p => {
      const isMatch = topKIds.has(p.id);
      ctx.fillStyle = isMatch ? '#10b981' : '#6366f1';
      ctx.beginPath();
      ctx.arc(p.x, p.y, isMatch ? 8 : 6, 0, Math.PI * 2);
      ctx.fill();

      if (isMatch) {
        ctx.strokeStyle = '#ffffff';
        ctx.lineWidth = 2;
        ctx.stroke();
      }

      // Label
      ctx.fillStyle = '#cbd5e1';
      ctx.font = '11px Inter, sans-serif';
      ctx.fillText(`ID ${p.id}`, p.x + 10, p.y + 4);
    });

    // Draw Query Point
    ctx.fillStyle = '#ef4444';
    ctx.beginPath();
    ctx.arc(query.x, query.y, 9, 0, Math.PI * 2);
    ctx.fill();
    ctx.strokeStyle = '#ffffff';
    ctx.lineWidth = 2;
    ctx.stroke();

    // Query pulse ring
    ctx.strokeStyle = 'rgba(239, 68, 68, 0.35)';
    ctx.lineWidth = 4;
    ctx.beginPath();
    ctx.arc(query.x, query.y, 16, 0, Math.PI * 2);
    ctx.stroke();

    ctx.fillStyle = '#fca5a5';
    ctx.font = 'bold 12px Inter, sans-serif';
    ctx.fillText('Query', query.x + 12, query.y - 8);
  }

  function renderTable(topK, metric) {
    if (topK.length === 0) {
      resultsContainer.innerHTML = '<p style="color:var(--text-muted);font-size:0.88rem;">No points match the selected filter category.</p>';
      return;
    }

    let rows = topK.map((item, idx) => `
      <tr>
        <td><strong>#${idx + 1}</strong></td>
        <td><code>${item.id}</code></td>
        <td>${item.label}</td>
        <td><span class="badge" style="background:var(--bg-surface);padding:2px 8px;border-radius:4px;font-size:0.75rem;">${item.category}</span></td>
        <td><strong>${item.score.toFixed(4)}</strong></td>
        <td>${item.distance.toFixed(2)}</td>
      </tr>
    `).join('');

    resultsContainer.innerHTML = `
      <table class="table" style="margin-top:8px;">
        <thead>
          <tr>
            <th>Rank</th>
            <th>Point ID</th>
            <th>Metadata Title</th>
            <th>Category</th>
            <th>Similarity Score</th>
            <th>Raw Distance</th>
          </tr>
        </thead>
        <tbody>
          ${rows}
        </tbody>
      </table>
    `;
  }

  // Canvas Interactions
  function getPos(e) {
    const rect = canvas.getBoundingClientRect();
    const scaleX = canvas.width / rect.width;
    const scaleY = canvas.height / rect.height;
    return {
      x: (e.clientX - rect.left) * scaleX,
      y: (e.clientY - rect.top) * scaleY
    };
  }

  canvas.addEventListener('mousedown', (e) => {
    const pos = getPos(e);
    const dist = Math.hypot(pos.x - query.x, pos.y - query.y);
    if (dist < 25) {
      isDragging = true;
    } else {
      query = pos;
      runSearch();
    }
  });

  window.addEventListener('mousemove', (e) => {
    if (isDragging) {
      query = getPos(e);
      // clamp to canvas bounds
      query.x = Math.max(15, Math.min(canvas.width - 15, query.x));
      query.y = Math.max(15, Math.min(canvas.height - 15, query.y));
      runSearch();
    }
  });

  window.addEventListener('mouseup', () => {
    isDragging = false;
  });

  // Touch Support
  canvas.addEventListener('touchstart', (e) => {
    if (e.touches.length === 1) {
      const touch = e.touches[0];
      const pos = getPos(touch);
      query = pos;
      runSearch();
      e.preventDefault();
    }
  }, { passive: false });

  canvas.addEventListener('touchmove', (e) => {
    if (e.touches.length === 1) {
      const touch = e.touches[0];
      query = getPos(touch);
      runSearch();
      e.preventDefault();
    }
  }, { passive: false });

  // Control Listeners
  metricSelect.addEventListener('change', runSearch);
  kSlider.addEventListener('input', (e) => {
    kVal.textContent = e.target.value;
    runSearch();
  });
  filterTag.addEventListener('change', runSearch);

  btnAddRandom.addEventListener('click', () => {
    const newId = 100 + points.length + 1;
    const cats = ['ai', 'systems', 'cloud'];
    const chosenCat = cats[Math.floor(Math.random() * cats.length)];
    points.push({
      id: newId,
      x: Math.floor(Math.random() * (canvas.width - 80)) + 40,
      y: Math.floor(Math.random() * (canvas.height - 80)) + 40,
      label: `Doc ${newId}: Random Vector Point`,
      category: chosenCat
    });
    runSearch();
  });

  btnReset.addEventListener('click', () => {
    points = [
      { id: 101, x: 180, y: 120, label: 'Doc 101: Rust Concurrency', category: 'systems' },
      { id: 102, x: 220, y: 140, label: 'Doc 102: HNSW Graphs', category: 'ai' },
      { id: 103, x: 450, y: 310, label: 'Doc 103: Cloud Kubernetes', category: 'cloud' },
      { id: 104, x: 280, y: 170, label: 'Doc 104: Vector Embeddings', category: 'ai' },
      { id: 105, x: 520, y: 260, label: 'Doc 105: Postgres WAL Logs', category: 'systems' },
      { id: 106, x: 420, y: 360, label: 'Doc 106: Serverless Architecture', category: 'cloud' },
      { id: 107, x: 160, y: 230, label: 'Doc 107: SIMD AVX2 Intrinsics', category: 'systems' },
      { id: 108, x: 310, y: 220, label: 'Doc 108: Transformers & RAG', category: 'ai' }
    ];
    query = { x: 240, y: 160 };
    metricSelect.value = 'cosine';
    kSlider.value = '3';
    kVal.textContent = '3';
    filterTag.value = 'all';
    runSearch();
  });

  // Initial Run
  runSearch();
}
