# Hướng dẫn sử dụng `repo` — Quản lý đa kho git

`repo` là công cụ dòng lệnh giúp bạn quản lý nhiều kho git cùng lúc từ một thư mục workspace. Thay vì phải chạy `git pull` từng kho một, bạn có thể chọn nhiều kho và thực hiện thao tác chỉ trong một lệnh.

---

## Mục lục

1. [Cài đặt](#cài-đặt)
2. [Cấu trúc workspace](#cấu-trúc-workspace)
3. [Các lệnh](#các-lệnh)
4. [Ví dụ thực tế](#ví-dụ-thực-tế)
5. [Gỡ lỗi thường gặp](#gỡ-lỗi-thường-gặp)

---

## Cài đặt

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

Chuyển sang một nhánh trong các kho được chọn.

```bash
repo checkout develop
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
