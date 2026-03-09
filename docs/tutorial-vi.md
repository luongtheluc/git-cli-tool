# Hướng dẫn sử dụng `repo` — Quản lý đa kho git

`repo` là công cụ dòng lệnh giúp bạn quản lý nhiều kho git cùng lúc từ một thư mục workspace. Thay vì phải chạy `git pull` từng kho một, bạn có thể chọn nhiều kho và thực hiện thao tác chỉ trong một lệnh.

---

## Mục lục

1. [Cài đặt — Windows](#cài-đặt--windows)
2. [Cài đặt — Ubuntu/Linux](#cài-đặt--ubuntulinux)
3. [Cấu trúc workspace](#cấu-trúc-workspace)
4. [Các lệnh](#các-lệnh)
5. [Ví dụ thực tế](#ví-dụ-thực-tế)
6. [Gỡ lỗi thường gặp](#gỡ-lỗi-thường-gặp)
7. [Tham khảo lệnh](#tham-khảo-lệnh)

---

## Cài đặt — Windows

### Bước 1: Build từ source

Yêu cầu: [Rust toolchain](https://rustup.rs) đã được cài đặt.

```bash
git clone <repo-url>
cd git-cli-tool
cargo build --release
```

Sau khi build xong, hai file nhị phân được tạo ra trong `target/release/`:
- `repo.exe` — công cụ chính
- `setup.exe` — trình cài đặt tự động

### Bước 2: Chạy trình cài đặt

```bash
.\target\release\setup.exe
```

Trình cài đặt sẽ:
1. Sao chép `repo.exe` vào `%USERPROFILE%\.repo\bin\` (Windows) hoặc `~/.repo/bin/` (Unix)
2. Tự động thêm thư mục đó vào biến môi trường `PATH` (ghi vào registry trên Windows, ghi vào `~/.profile` trên Unix)

Ví dụ đầu ra khi cài đặt thành công:

```
repo Setup Installer
────────────────────────────────────────

  Source : C:\...\target\release\repo.exe
  Target : C:\Users\ten\.repo\bin\repo.exe

✓ Installed C:\Users\ten\.repo\bin\repo.exe
✓ Added to user PATH (HKCU\Environment)

Installation complete!
  Restart your terminal, then run: repo --help
```

### Bước 3: Xác nhận cài đặt

Khởi động lại terminal, sau đó chạy:

```bash
repo --help
```

---

## Cài đặt — Ubuntu/Linux

Có hai cách: **cross-compile từ Windows** hoặc **build thẳng trên Ubuntu**.

### Cách A: Cross-compile từ Windows (yêu cầu Docker Desktop)

**Bước 1:** Build binary Linux từ máy Windows:

```bat
installer\build-linux.bat
```

Script sẽ tự cài `cross` (nếu chưa có) và tạo file:
```
target\x86_64-unknown-linux-gnu\release\repo
```

**Bước 2:** Copy file `repo` sang máy Ubuntu, rồi chạy script cài đặt:

```bash
bash installer/install-ubuntu.sh
```

### Cách B: Build thẳng trên Ubuntu (yêu cầu Rust)

```bash
# Cài Rust nếu chưa có
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Clone và build
git clone <repo-url>
cd git-cli-tool
cargo build --release

# Cài đặt và thiết lập alias
bash installer/install-ubuntu.sh
```

### Kết quả sau khi cài đặt

Script `install-ubuntu.sh` sẽ:
1. Sao chép binary vào `~/.local/bin/repo`
2. Tự động thêm `alias repo="$HOME/.local/bin/repo"` vào `~/.bashrc` và `~/.zshrc`

Ví dụ đầu ra:

```
  repo — Ubuntu installer
  ─────────────────────────────────────
   Binary  : target/release/repo
   Install : /home/ten/.local/bin/repo

✓ Installed /home/ten/.local/bin/repo
✓ Added alias to /home/ten/.bashrc
✓ Added alias to /home/ten/.zshrc

  Installation complete!
  Reload your shell or run:
    source ~/.bashrc
  Then: repo --help
```

**Bước cuối:** Reload shell và kiểm tra:

```bash
source ~/.bashrc   # hoặc source ~/.zshrc
repo --help
```

---

## Cấu trúc workspace

`repo` quét thư mục hiện tại để tìm các kho git là **con trực tiếp** (depth 1). Workspace nên có cấu trúc như sau:

```
workspace/
├── api-service/       ← chứa .git
├── auth-service/      ← chứa .git
├── frontend/          ← chứa .git
└── worker/            ← chứa .git
```

Chạy `repo` từ thư mục `workspace/`. Các kho lồng sâu hơn sẽ không được phát hiện.

---

## Các lệnh

### `repo ui` — Giao diện TUI tương tác

Khởi chạy giao diện terminal đầy đủ màn hình theo phong cách lazygit — duyệt kho và xem trạng thái git mà không cần gõ lệnh.

```bash
repo ui
```

**Bố cục màn hình:**

```
┌─ REPOS ──────────────┐┌─ api-service [main] ─────────────────────┐
│▶ ● api-service  main ││   M  modified  src/main.rs               │
│  ● frontend     dev  ││  ??  untracked config/local.toml         │
│    worker       main ││   A  added     tests/integration_test.rs │
└──────────────────────┘└──────────────────────────────────────────┘
│ ↑↓ Navigate   r Refresh  Tab Focus   q Quit  │  3 changed file(s)│
```

**Màu sắc trạng thái (giống lazygit):**
- Xanh lá `green` — file đã staged (chờ commit)
- Vàng `yellow` — file đã sửa nhưng chưa staged
- Đỏ `red` — file chưa được track (untracked)
- Vàng nhạt `●` — kho có thay đổi chưa commit

**Phím tắt:**

| Phím | Chức năng |
|------|-----------|
| `↑` / `↓` hoặc `j` / `k` | Di chuyển lên/xuống danh sách kho |
| `r` hoặc `F5` | Làm mới trạng thái kho đang chọn |
| `Tab` | Chuyển focus giữa sidebar và panel chính |
| `q` hoặc `Esc` | Thoát TUI |
| `Ctrl+C` | Thoát khẩn cấp |

---

### `repo list` — Xem danh sách kho

Hiển thị tất cả kho được phát hiện cùng với nhánh hiện tại và commit gần nhất.

```bash
repo list
```

Ví dụ đầu ra:

```
REPO              BRANCH        LAST COMMIT
──────────────────────────────────────────────────────────────
api-service       develop       a12bc3 fix auth bug
auth-service      main          7a91de update deps          *
frontend          feature/ui    0ab221 add layout
worker            develop       c2f991 fix queue
```

Dấu `*` ở cuối dòng nghĩa là kho đó có thay đổi chưa được commit.

---

### `repo checkout <branch>` — Chuyển nhánh

Chuyển sang một nhánh trong các kho được chọn. Thêm `-b` để tạo nhánh mới nếu chưa tồn tại.

```bash
repo checkout develop           # chuyển sang nhánh đã có
repo checkout feature/login -b  # tạo mới và chuyển sang nhánh đó
```

Sau khi chạy, giao diện chọn kho xuất hiện (phím cách để chọn/bỏ chọn, Enter để xác nhận):

```
Select repositories (space to toggle, enter to confirm):
> [x] api-service      develop      a12bc3 fix auth bug
  [x] auth-service     main         7a91de update deps
  [ ] frontend         feature/ui   0ab221 add layout
  [x] worker           develop      c2f991 fix queue
```

Sau khi xác nhận, lệnh được thực thi lần lượt trên từng kho đã chọn:

```
=== api-service ===
Already on 'develop'

=== auth-service ===
Switched to branch 'develop'

=== worker ===
Already on 'develop'
```

---

### `repo pull` — Lấy code mới nhất

Chạy `git pull` trên các kho được chọn.

```bash
repo pull
```

Giao diện chọn kho hiện ra → chọn các kho cần pull → Enter.

```
=== api-service ===
Already up to date.

=== auth-service ===
Updating 7a91de..c3f210
Fast-forward
 src/auth.rs | 12 +++---
 1 file changed, 6 insertions(+), 6 deletions(-)
```

Nếu một kho gặp lỗi (ví dụ không có remote), công cụ in lỗi màu đỏ và tiếp tục xử lý các kho còn lại.

---

### `repo push` — Đẩy code lên remote

Chạy `git push` trên các kho được chọn.

```bash
repo push
```

---

### `repo commit -m "<message>"` — Commit thay đổi

Stage tất cả thay đổi (`git add -A`) rồi commit với message cho trước, trên các kho được chọn.

```bash
repo commit -m "fix: update API response format"
```

Ví dụ đầu ra:

```
=== api-service ===
[develop 3a9f21c] fix: update API response format
 2 files changed, 15 insertions(+), 3 deletions(-)

=== worker ===
On branch develop
nothing to commit, working tree clean
```

> **Lưu ý:** Lệnh này chạy `git add -A` trước, tức là tất cả file thay đổi (kể cả file mới) đều được stage. Hãy kiểm tra kỹ trước khi dùng.

---

### `repo run <script>` — Chạy script trên nhiều kho

Chạy một script hoặc lệnh trên các kho được chọn. Công cụ tự động nhận diện loại dự án từ file manifest và ánh xạ tên script sang lệnh thực tế.

```bash
repo run build
repo run test
repo run build --jobs 4    # chạy tối đa 4 kho song song
repo run build --jobs 0    # tự động: số CPU / 2
```

**Tự động nhận diện build tool:**

| File manifest | Build tool | `build` → | `test` → |
|---|---|---|---|
| `package.json` | npm | `npm run build` | `npm test` |
| `Cargo.toml` | cargo | `cargo build --release` | `cargo test` |
| `go.mod` | Go | `go build ./...` | `go test ./...` |
| `Makefile` | make | `make build` | `make test` |
| *(không có)* | shell | chạy trực tiếp | — |

**Tùy chọn `--jobs` (`-j`):**
- `--jobs 1` *(mặc định)* — chạy tuần tự, an toàn
- `--jobs 0` — tự động: `max(1, số_CPU / 2)`
- `--jobs N` — giới hạn tối đa N kho chạy cùng lúc

Ví dụ đầu ra:

```
  Running build on 3 repo(s) with 2 job(s)...

[api-service] > cargo build --release
[api-service]    Compiling api-service v0.1.0
[api-service]    Finished `release` profile
[frontend] > npm run build
[frontend] > vite build ...
[frontend]   dist/index.html  2.10 kB
[worker] > go build ./...

  3 of 3 completed successfully.
```

Khi một kho thất bại, lỗi được in màu đỏ kèm prefix `[tên-kho]` và các kho còn lại vẫn tiếp tục chạy.

---

### `repo status` — Xem trạng thái

Chạy `git status` trên các kho được chọn.

```bash
repo status
```

Ví dụ đầu ra:

```
=== auth-service ===
On branch main
Changes not staged for commit:
  modified:   src/config.rs

=== frontend ===
On branch feature/ui
nothing to commit, working tree clean
```

---

## Ví dụ thực tế

### Kịch bản: Bắt đầu ngày làm việc

Buổi sáng bạn muốn đồng bộ tất cả kho về code mới nhất trên nhánh `develop`:

```bash
cd ~/workspace

# 1. Xem tình trạng hiện tại
repo list

# 2. Chuyển tất cả kho sang develop (chọn tất cả bằng cách nhấn a)
repo checkout develop

# 3. Pull code mới nhất cho tất cả kho đã chọn
repo pull
```

### Kịch bản: Commit đồng thời nhiều kho

Sau khi sửa một ticket ảnh hưởng đến nhiều service:

```bash
# Commit các kho liên quan với cùng một message
repo commit -m "feat: add user role validation"

# Đẩy lên remote
repo push
```

### Kịch bản: Build tất cả service sau khi pull

```bash
# Pull code mới
repo pull

# Build song song tối đa 4 kho cùng lúc
repo run build --jobs 4

# Chạy test trên tất cả kho
repo run test --jobs 0
```

### Kịch bản: Kiểm tra kho nào có thay đổi chưa commit

```bash
repo list
# Kho nào có dấu * ở cuối dòng là có thay đổi chưa commit

# Xem chi tiết trạng thái của các kho đó
repo status
```

---

## Gỡ lỗi thường gặp

### `repo: command not found`

PATH chưa được cập nhật. Hãy:
- Khởi động lại terminal (hoặc mở terminal mới)
- Kiểm tra bằng lệnh: `echo $PATH` (Unix) hoặc `echo %PATH%` (Windows CMD)
- Nếu `~/.repo/bin` (hoặc `%USERPROFILE%\.repo\bin`) chưa có trong PATH, chạy lại `setup.exe`

---

### `'repo.exe' not found in ...` khi chạy setup

File `repo.exe` chưa được build hoặc không cùng thư mục với `setup.exe`. Giải pháp:

```bash
cargo build --release
# Sau đó chạy setup từ đúng thư mục
.\target\release\setup.exe
```

---

### Lỗi `Failed to run git — is git installed and in PATH?`

Git chưa được cài hoặc không có trong PATH. Kiểm tra:

```bash
git --version
```

Nếu git chưa cài, tải tại [git-scm.com](https://git-scm.com).

---

### Kho không xuất hiện trong danh sách

`repo` chỉ quét **con trực tiếp** của thư mục hiện tại (depth 1). Đảm bảo:
- Bạn đang chạy lệnh từ thư mục workspace (không phải từ bên trong một kho)
- Kho cần quản lý nằm ngay trong workspace, không lồng sâu hơn

Ví dụ cấu trúc **đúng**:
```
workspace/        ← chạy repo từ đây
└── my-service/
    └── .git
```

Ví dụ cấu trúc **sai** (không được phát hiện):
```
workspace/        ← chạy repo từ đây
└── group/
    └── my-service/
        └── .git  ← quá sâu, bị bỏ qua
```

---

### Lỗi màu đỏ khi pull/push nhưng các kho khác vẫn chạy bình thường

Đây là hành vi mặc định: khi một kho thất bại, công cụ in lỗi màu đỏ và tiếp tục xử lý các kho còn lại. Bạn không cần lo lắng — hãy đọc thông báo lỗi để hiểu nguyên nhân (thường là xung đột, không có remote, hoặc chưa có quyền push).

---

### `repo run` không nhận diện đúng build tool

Kiểm tra xem file manifest có tồn tại ngay trong thư mục gốc của kho không (không phải thư mục con). Thứ tự ưu tiên: `package.json` > `Cargo.toml` > `go.mod` > `Makefile`. Nếu kho có nhiều loại manifest, loại được phát hiện đầu tiên sẽ được dùng.

---

## Tham khảo lệnh

| Lệnh | Mô tả |
|------|-------|
| `repo ui` | Giao diện TUI tương tác (lazygit-style) |
| `repo list` | Hiển thị tất cả kho với nhánh và commit gần nhất |
| `repo checkout <branch> [-b]` | Chuyển nhánh; `-b` để tạo mới nếu chưa tồn tại |
| `repo pull` | Pull từ remote trên các kho được chọn |
| `repo push` | Push lên remote trên các kho được chọn |
| `repo commit -m "<msg>"` | Stage tất cả + commit trên các kho được chọn |
| `repo status` | Xem trạng thái git chi tiết của các kho được chọn |
| `repo run <script> [--jobs N]` | Chạy script với tự động nhận diện build tool |
