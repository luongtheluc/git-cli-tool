// Fade-in on scroll using IntersectionObserver
document.addEventListener('DOMContentLoaded', () => {
  const observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          entry.target.classList.add('visible');
          observer.unobserve(entry.target);
        }
      });
    },
    { threshold: 0.1 }
  );

  document.querySelectorAll('.fade-in').forEach((el) => observer.observe(el));

  // Install tabs switching with null-safe lookup
  document.querySelectorAll('.tab-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
      const target = document.getElementById(btn.dataset.tab);
      if (!target) return;
      document.querySelectorAll('.tab-btn').forEach((b) => {
        b.classList.remove('active');
        b.setAttribute('aria-selected', 'false');
      });
      document.querySelectorAll('.tab-content').forEach((c) => c.classList.remove('active'));
      btn.classList.add('active');
      btn.setAttribute('aria-selected', 'true');
      target.classList.add('active');
    });
  });

  // Mobile nav toggle
  const navToggle = document.querySelector('.nav-toggle');
  const navLinks = document.querySelector('.nav-links');
  if (navToggle && navLinks) {
    navToggle.addEventListener('click', () => {
      navLinks.classList.toggle('open');
    });
  }

  // Language toggle (EN ↔ VI)
  const langBtn = document.getElementById('lang-toggle');
  if (langBtn) {
    let currentLang = localStorage.getItem('repo-lang') || 'en';
    // Apply saved language on load
    if (currentLang === 'vi') {
      setLanguage('vi');
      langBtn.textContent = 'EN';
    }
    langBtn.addEventListener('click', () => {
      currentLang = currentLang === 'en' ? 'vi' : 'en';
      setLanguage(currentLang);
      langBtn.textContent = currentLang === 'en' ? 'VI' : 'EN';
    });
  }
});
