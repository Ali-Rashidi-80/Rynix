# Install Rynix (فارسی)

**زبان‌ها:** [English](INSTALL.md) (پیش‌فرض) · [فارسی](INSTALL.fa.md)

> همراهِ [`INSTALL.md`](INSTALL.md). در تعارض، نسخهٔ انگلیسی اولویت دارد.

نصب **`rynixc`** از سورس یا GitHub Release. هر گام زیر فرمان verify هم‌تراز CI دارد.

## پیش‌نیازها

| نیاز | یادداشت |
|------|---------|
| **Rust** | MSRV **1.98** |
| **clang** | برای `build` / `run` / `emit-wasm` (یک‌مسیره، پایین) |
| **Python 3** | اختیاری — فقط Suite5 |
| **Node.js 18+** | اختیاری — افزونه VS Code و smokeهای wasm |

### یک‌مسیره clang (ویندوز + لینوکس)

| پلتفرم | راه‌اندازی |
|--------|------------|
| **ویندوز** | LLVM/clang یا MinGW-w64 clang روی `PATH`؛ ترجیح `x86_64-pc-windows-gnu`؛ `--runtime=portable` |
| **لینوکس** | `clang` توزیع؛ `--runtime=portable` یا `uring` |

```sh
clang --version
rynixc build examples/01_hello.ryx -o target/hello --runtime=portable
```

جزئیات کامل اسکریپت‌ها، verify و عیب‌یابی: متن انگلیسی [`INSTALL.md`](INSTALL.md).
