// Vietnamese translations for the landing page
// Keys match data-i18n attributes in index.html
const translations = {
  vi: {
    // Navigation
    'nav.features': 'Tính năng',
    'nav.demo': 'Demo',
    'nav.install': 'Cài đặt',
    'nav.commands': 'Lệnh',
    'nav.tutorial': 'Hướng dẫn',

    // Hero
    'hero.tagline': 'Quản lý nhiều kho Git từ một thư mục workspace. Khám phá, kiểm tra và thao tác hàng loạt trên các kho với giao diện terminal tương tác.',
    'hero.download': 'Tải bản mới nhất',
    'hero.github': 'Xem trên GitHub',

    // Features
    'feat.discovery.title': 'Tự động phát hiện',
    'feat.discovery.desc': 'Tự động tìm tất cả kho Git trong thư mục workspace. Không cần file cấu hình.',
    'feat.tui.title': 'Giao diện TUI tương tác',
    'feat.tui.desc': 'Chọn nhiều kho bằng phím cách, xác nhận bằng Enter. Thao tác hàng loạt an toàn, trực quan.',
    'feat.scan.title': 'Quét song song',
    'feat.scan.desc': 'Quét 50+ kho trong dưới 2 giây nhờ xử lý song song Rayon.',
    'feat.batch.title': 'Thao tác hàng loạt',
    'feat.batch.desc': 'Checkout, pull, push, commit và status trên nhiều kho chỉ trong một lệnh.',
    'feat.error.title': 'Xử lý lỗi linh hoạt',
    'feat.error.desc': 'Một kho lỗi không dừng cả batch. Báo lỗi chi tiết từng kho giúp bạn nắm rõ tình hình.',
    'feat.cross.title': 'Đa nền tảng',
    'feat.cross.desc': 'Chạy trên Windows, macOS và Linux. Binary native với đầu ra màu ANSI.',

    // Commands table
    'cmd.header.cmd': 'Lệnh',
    'cmd.header.desc': 'Mô tả',
    'cmd.list': 'Liệt kê tất cả kho với nhánh, commit gần nhất, trạng thái thay đổi',
    'cmd.checkout': 'Chuyển nhánh trên các kho đã chọn',
    'cmd.pull': 'Pull code mới nhất trên các kho đã chọn',
    'cmd.push': 'Push commit lên remote trên các kho đã chọn',
    'cmd.commit': 'Stage tất cả + commit trên các kho đã chọn',
    'cmd.status': 'Xem trạng thái git trên các kho đã chọn',
    'cmd.git': 'Chạy bất kỳ lệnh git nào trên các kho đã chọn',
    'cmd.run': 'Chạy script build trên các kho đã chọn (tự nhận diện build tool)',
    'cmd.ui': 'Mở giao diện TUI tương tác',

    // Tutorial
    'tut.1.title': 'Thiết lập workspace',
    'tut.1.desc': 'Tạo thư mục và clone các kho vào đó như thư mục con trực tiếp:',
    'tut.2.title': 'Xem danh sách kho',
    'tut.2.desc': 'Xem tổng quan tất cả kho — nhánh, commit gần nhất và trạng thái thay đổi:',
    'tut.3.title': 'Chuyển nhánh + pull hàng loạt',
    'tut.3.desc': 'Chuyển tất cả kho sang <code>develop</code> và pull code mới nhất. Chọn kho bằng <kbd>Space</kbd>, xác nhận bằng <kbd>Enter</kbd>:',
    'tut.4.title': 'Commit và push nhiều kho',
    'tut.4.desc': 'Stage tất cả thay đổi và commit với cùng một message, sau đó push:',
    'tut.5.title': 'Chạy lệnh git tùy ý',
    'tut.5.desc': 'Truyền lệnh git bất kỳ cho các kho đã chọn với <code>repo git</code>:',
    'tut.6.title': 'Build và test song song',
    'tut.6.desc': 'Tự nhận diện build tool (npm, cargo, go, make) cho từng kho:',
    'tut.7.title': 'Mở giao diện TUI',
    'tut.7.desc': 'Dashboard toàn màn hình với trình xem trạng thái và thao tác hàng loạt — phong cách lazygit:',
  }
};

// Store original English text on first load
const originalTexts = {};

/**
 * Apply translations for the given language.
 * 'en' restores original text; 'vi' applies Vietnamese translations.
 */
function setLanguage(lang) {
  document.querySelectorAll('[data-i18n]').forEach((el) => {
    const key = el.getAttribute('data-i18n');

    // Save original English text on first encounter
    if (!originalTexts[key]) {
      originalTexts[key] = el.innerHTML;
    }

    if (lang === 'vi' && translations.vi[key]) {
      el.innerHTML = translations.vi[key];
    } else {
      el.innerHTML = originalTexts[key];
    }
  });

  document.documentElement.lang = lang;
  localStorage.setItem('repo-lang', lang);
}
