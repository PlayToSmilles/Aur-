# Aruã Browser

Protótipo Windows em Rust, com interpretação de HTML e fluxo de blocos próprios.
A Mark I abre um arquivo UTF-8 local e desenha texto com Win32, Direct2D e
DirectWrite. Ainda não é um navegador compatível com a web moderna.

## Compilar e testar

Na pasta `C:\Dev\Arua`, com a toolchain `stable-x86_64-pc-windows-msvc`:

```powershell
cargo test --locked
cargo build --release --locked --target x86_64-pc-windows-msvc
.\target\x86_64-pc-windows-msvc\release\Arua.exe
```

Sem argumento, abre `examples\mark_i.html` do checkout usado na compilação.
Para selecionar outro documento:

```powershell
.\target\x86_64-pc-windows-msvc\release\Arua.exe "C:\caminho\documento.html"
```

HTML aceito: estrutura explícita `html`, `head` opcional, `title` opcional no
head, `body` e blocos `h1`/`p` com texto puro. Não aceita atributos, CSS, scripts,
imagens ou elementos desconhecidos. Leia o contrato completo e os resultados
em [docs/MARK_I.md](docs/MARK_I.md).

O programa `validation\mark0-smoke` e os arquivos da Mark 0 foram preservados.
O repositório não contém Chromium, WebKit, Gecko, Servo, WebView2 ou Electron.
