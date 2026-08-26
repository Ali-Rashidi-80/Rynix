# Contributing (فارسی)

**زبان‌ها:** [English](CONTRIBUTING.md) (پیش‌فرض) · [فارسی](CONTRIBUTING.fa.md)

> همراهِ [`CONTRIBUTING.md`](CONTRIBUTING.md). انگلیسی کانونیکال است.

1. تغییرات اتمی و تست‌شده (`cargo test --workspace`، clippy `-D warnings`).
2. ویژگی زبان را بدون SPEC + تست در README اختراع نکنید.
3. ترجیح: اصلاح کامپایلر به‌جای ضعیف کردن تست.
4. تغییر رانتایم: `rt/tests` / `size_echo_gates` را اجرا کنید.
5. راهنمای عامل: [AGENTS.md](AGENTS.md) · [AGENTS.fa.md](AGENTS.fa.md).
6. لایسنس دوگانه MIT OR Apache-2.0.
7. trailerهای `Co-authored-by: Cursor <…>` اضافه نکنید؛ اختیاری: `git config core.hooksPath .githooks`.

## مستندات دوزبانه

- **انگلیسی** = منبع حقیقت (به‌ویژه SPEC، ADR، اسکیماها).
- فایل‌های `.fa.md` همراه ورود/نصب/مشارکت/امنیت/عامل هستند و باید با واقعیت درخت هم‌خوان بمانند.
- ROADMAP ✅ فقط با تست درختی.
- Suite5: افشای strength reduction اجباری است.
