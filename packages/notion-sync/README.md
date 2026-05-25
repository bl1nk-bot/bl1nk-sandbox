# notion-sync (`ons`)

Rust CLI สำหรับ sync .md files จาก Obsidian vault ขึ้น Notion database

## Build

```bash
cargo build --release
# binary อยู่ที่ target/release/ons
```

## Setup

แก้ `settings.json`:

```json
{
  "notion_token": "secret_xxx",
  "notion_database_id": "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
  "notion_version": "2022-06-28",
  "vault_path": "/Users/you/Obsidian/MyVault",
  "vault_subfolder": "Prompts"
}
```

- `notion_token` — Notion Integration token จาก https://www.notion.so/my-integrations
- `notion_database_id` — ID จาก URL ของ database (ส่วนที่เป็น 32 chars)
- `notion_version` — Notion API version (ดูจาก docs.notion.com)
- `vault_subfolder` — optional, ถ้าไม่ใส่จะ scan vault ทั้งหมด

## Frontmatter template สำหรับ .md ใน Obsidian

```yaml
---
title: "ชื่อ prompt"
id: PROMPT-001
description: "คำอธิบายสั้น"
status: unused
source: original
created: 2026-05-25
updated: 2026-05-25
tags:
  - ai
  - revenue
used_in_post: null
scheduled_post: null
---

เนื้อหา prompt ที่นี่...
```

## Commands

```bash
# ตรวจสอบ settings.json
ons check

# ดู parsed frontmatter ของ file
ons inspect path/to/file.md

# sync ทั้ง vault (dry-run ก่อน)
ons sync --dry-run
ons sync

# push file เดียว
ons push path/to/file.md --dry-run
ons push path/to/file.md

# ใช้ config อื่น
ons --config other-settings.json sync
```

## Logic

- ถ้า `File_name` ยังไม่มีใน Notion → **create** page ใหม่
- ถ้ามีแล้ว → **update** properties + replace body blocks
- File ที่ไม่มี frontmatter → **skip**
- Field ใหม่ที่เพิ่มมาใน frontmatter ภายหลัง → sync ได้ทันที ไม่ต้องแก้ code
