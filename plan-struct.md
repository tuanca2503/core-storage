# Core Storage Workspace Structure

## Overview

Workspace được chia theo trách nhiệm (Responsibility Separation).

Mỗi crate chỉ đảm nhiệm **một nhiệm vụ duy nhất**, tránh việc business logic, transport và persistence phụ thuộc lẫn nhau.

```
CLI / HTTP / TCP / WebSocket
                │
                ▼
            Service
          ┌─────┴─────┐
          ▼           ▼
      Database      Core
          │           │
          └─────┬─────┘
                ▼
             Platform
```

---

# Crates

```
crates/
├── api
├── cli
├── core
├── database
├── model
├── platform
└── service
```

---

# api

## Responsibility

Là tầng giao tiếp với bên ngoài (Transport Layer).

API chỉ chịu trách nhiệm:

- Nhận request
- Parse request
- Authenticate (nếu có)
- Gọi Service
- Chuyển kết quả thành HTTP/TCP/WebSocket Response

API **không chứa business logic**.

## Structure

```
api/
└── src/
    ├── http/
    ├── tcp/
    ├── websocket/
    └── lib.rs
```

## Ví dụ

HTTP

```
POST /format
```

↓

```
FormatDiskRequest
```

↓

```
StorageService::format_disk(...)
```

↓

```
HTTP Response
```

---

# cli

## Responsibility

Command Line Interface.

CLI chỉ:

- Parse command line
- Parse option
- Kết nối TCP Client
- Hiển thị kết quả

Không chứa business logic.

Ví dụ:

```
core-storage disk list
```

↓

```
TcpClient
```

↓

```
Storage Service
```

CLI quyết định:

- Table
- JSON
- Pretty Print

---

# service

## Responsibility

Business Logic.

Đây là nơi xử lý toàn bộ nghiệp vụ.

Ví dụ:

- Kiểm tra quyền
- Kiểm tra force
- Kiểm tra disk có được format
- Kiểm tra disk đang mounted
- Validate nghiệp vụ
- Điều phối Database + Core

Service **không biết**:

- HTTP
- TCP
- CLI
- JSON
- Table

Ví dụ:

```
format_disk(request)
```

Service sẽ:

```
Validate

↓

Load Disk

↓

Check Permission

↓

Check Force

↓

Call Core

↓

Update Database
```

---

# core

## Responsibility

Storage Engine.

Core chịu trách nhiệm:

- Header
- Chunk
- Segment
- Object
- Bitmap
- Allocation
- Read
- Write
- Format
- Mount

Core không biết:

- HTTP
- Database
- CLI

Core chỉ xử lý Storage Engine.

---

# platform

## Responsibility

Platform Abstraction Layer.

Chỉ thao tác với hệ điều hành.

Ví dụ:

Windows

- Raw Device
- Volume
- Physical Disk

Linux

- /dev/*
- mount
- umount
- ioctl

Platform không biết:

- Chunk
- Segment
- Header
- Object

Platform chỉ biết cách đọc ghi thiết bị.

---

# database

## Responsibility

Persistence Layer.

Chịu trách nhiệm:

- SQLite Manager
- Transaction
- Execute SQL
- Mapping database

Không có business logic.

Ví dụ:

```
Service

↓

Database

↓

SQLite
```

Database không quyết định:

- Có được format hay không
- Có được mount hay không

---

# model

## Responsibility

Chứa toàn bộ Domain Model dùng chung.

Các crate:

- Service
- Core
- Database

đều sử dụng model này.

## Structure

```
model/
└── src/
    ├── chunk.rs
    ├── segment.rs
    ├── header.rs
    ├── object.rs
    ├── disk.rs
    ├── volume.rs
    │
    ├── db/
    │   ├── chunk.rs
    │   ├── segment.rs
    │   ├── header.rs
    │   └── ...
    │
    └── lib.rs
```

## Domain Model

Ví dụ:

```
Chunk
Segment
Header
Object
Disk
```

Đây là model của toàn hệ thống.

Không thuộc:

- Platform
- Database

---

## db/

Không phải DTO.

Đây là Persistence Helper.

Ví dụ:

```
Chunk
```

↓

```
db::chunk
```

chứa:

- CREATE TABLE
- INSERT
- UPDATE
- DELETE
- SELECT
- Mapping

Nếu Database Schema giống Domain Model thì không cần tạo thêm:

```
ChunkRow
```

Database sẽ thao tác trực tiếp với Domain Model.

---

# Dependency

```
cli
    │
    ▼
api
    │
    ▼
service
   ├─────────────┐
   ▼             ▼
database       core
   │             │
   └──────┬──────┘
          ▼
        model

core
    ▼
platform
```

---

# Design Principles

## API

✔ Parse Request

✔ Parse Response

✔ Authentication

✔ Transport

❌ Business Logic

---

## Service

✔ Business Logic

✔ Validation

✔ Permission

✔ Workflow

❌ HTTP

❌ TCP

❌ SQL

❌ Table Output

---

## Core

✔ Storage Engine

✔ Read

✔ Write

✔ Allocation

✔ Header

✔ Chunk

✔ Segment

❌ HTTP

❌ Database

---

## Platform

✔ OS API

✔ Raw Device

✔ Mount

✔ Volume

✔ Physical Disk

❌ Business Logic

---

## Database

✔ SQLite

✔ Transaction

✔ Execute SQL

✔ Persistence

❌ Business Logic

---

## Model

✔ Shared Domain Model

✔ Shared Enum

✔ Shared Error

✔ Shared Request/Response (nếu là domain)

✔ Persistence Helper (db)

❌ Business Logic