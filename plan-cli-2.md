# CLI Framework Plan

## Mục tiêu

Xây dựng một CLI framework đơn giản, không phụ thuộc thư viện bên ngoài.

CLI chỉ có nhiệm vụ:

- Parse command line arguments
- Validate command
- Validate arguments/options
- Gọi command handler
- Format output

Command handler chỉ tập trung xử lý nghiệp vụ.

---

# Luồng xử lý

```text
argv
 │
 ▼
Parser
 │
 ▼
CommandContext
 │
 ▼
Dispatcher
 │
 ├── Validate command
 ├── Validate arguments
 ├── Validate options
 │
 ▼
Command Handler
 │
 ▼
CommandResult
 │
 ▼
Output Processor
 │
 ▼
Console
```

---

# Thư mục

```text
core-storage-cli
│
├── src
│   ├── cli
│   │   ├── parser.rs
│   │   ├── dispatcher.rs
│   │   ├── context.rs
│   │   ├── command.rs
│   │   ├── result.rs
│   │   └── mod.rs
│   │
│   ├── commands
│   │   ├── list.rs
│   │   ├── format.rs
│   │   ├── info.rs
│   │   └── mod.rs
│   │
│   └── main.rs
```

---

# Parser

Parser chỉ có nhiệm vụ đọc argv.

Ví dụ

```text
corestorage format disk0 --force --segment-size=64G
```

Parse thành

```text
CommandContext

command:
    format

arguments:
    disk0

options:
    force
    segment-size=64G
```

Parser KHÔNG kiểm tra:

- command có tồn tại không
- option hợp lệ không
- đủ tham số không

---

# CommandContext

Lưu toàn bộ dữ liệu sau khi parse.

```rust
pub struct CommandContext {

    pub command: String,

    pub arguments: Vec<Argument>,

    pub options: Vec<OptionItem>,
}
```

Helper:

```rust
arg(index)

arg_count()

has_option(name)

option(name)
```

---

# Command

Mỗi command khai báo metadata.

Ví dụ

```rust
pub struct Command {

    pub name: &'static str,

    pub description: &'static str,

    pub min_arguments: usize,

    pub max_arguments: usize,

    pub options: &'static [&'static str],

    pub handler: fn(CommandContext) -> CommandResult,
}
```

Ví dụ

```text
format

arguments

    disk

options

    force

    quick

    segment-size
```

---

# Dispatcher

Dispatcher thực hiện:

## 1. Tìm command

Nếu không tồn tại

```text
Unknown command.
```

---

## 2. Validate arguments

Ví dụ

```text
corestorage format
```

=> thiếu disk

```text
Usage:

corestorage format <disk>
```

---

## 3. Validate options

Ví dụ

```text
corestorage format disk0 --abc
```

=> báo lỗi

```text
Unknown option '--abc'
```

---

## 4. Gọi handler

```text
handler(context)
```

---

# Command Handler

Command handler chỉ xử lý nghiệp vụ.

Ví dụ

```rust
fn format(ctx)
```

không cần kiểm tra

- command
- argument count
- option hợp lệ

Dispatcher đã xử lý.

---

# Output

Handler không in trực tiếp.

Handler chỉ trả dữ liệu.

Ví dụ

```rust
CommandResult::Text

CommandResult::Table

CommandResult::List

CommandResult::Error
```

Output Processor quyết định cách hiển thị.

---

# Global Options

Global option áp dụng cho mọi command.

Ví dụ

```text
--json

--pretty

--verbose

--color

--sort
```

Các option này được Output Processor hoặc Dispatcher xử lý.

Command handler không cần biết.

Ví dụ

```text
corestorage list --json
```

Luồng

```text
list()

↓

ListResult

↓

JSON Output
```

---

# Command Options

Các option ảnh hưởng đến nghiệp vụ.

Ví dụ

```text
format

--force

--quick

repair

--deep

mount

--readonly
```

Các option này được truyền vào handler.

---

# Nguyên tắc

## Parser

Chỉ parse.

Không validate.

---

## Dispatcher

Validate.

Điều hướng command.

---

## Handler

Chỉ xử lý nghiệp vụ.

---

## Output

Chỉ render kết quả.

Không chứa nghiệp vụ.

---

# Giai đoạn phát triển

## Phase 1

- Parse command
- Parse arguments
- Parse options
- Dispatcher
- list command

---

## Phase 2

- Metadata command
- Validate argument count
- Validate option
- Usage message

---

## Phase 3

- Global options
- Output formatter
- JSON output
- Pretty table

---

## Phase 4

- Command alias
- Help command
- Version command
- Auto generated usage