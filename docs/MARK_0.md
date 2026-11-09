# Aruã Browser — Mark 0: preparação da oficina

Data: 9 de novembro de 2026 (America/Sao_Paulo).

Status: concluída. Ambiente Rust para Windows x64 instalado e validado por
compilação, linkedição e execução. A implementação do navegador não foi iniciada.

## Diretório oficial e escopo

O diretório oficial de desenvolvimento é `C:\Dev\Arua`, fora do OneDrive.
O Git foi inicializado nessa raiz, com a branch `main`. Não há commits nem
remote configurado nesta etapa. Os arquivos novos permanecem não rastreados
até serem adicionados ao índice em uma etapa posterior.

Não foi criado um pacote Cargo do navegador na raiz. O único projeto Rust é
um programa mínimo de validação em `validation\mark0-smoke`, sem dependências
externas. Seus arquivos e o executável foram preservados como evidência local.

## Instalação do Rust

Fonte oficial: <https://rust-lang.org/tools/install/>.

Instalador utilizado:
<https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe>.

O SHA-256 do download foi comparado com o arquivo `.sha256` publicado no
mesmo servidor oficial, antes da execução:

```text
38027D51D69A5D05493627A0634DBEA52D6CD793C8F072D2373D3F4163EEC7DB
```

Comando executado na pasta de desenvolvimento:

```powershell
.\.mark0\rustup-init.exe -y --default-host x86_64-pc-windows-msvc --default-toolchain stable --profile minimal
```

O perfil mínimo instalou Cargo, rustc e a biblioteca padrão para o alvo MSVC.
Não foram instalados motores web ou dependências de navegador.

Instalações por usuário:

- Ferramentas/proxies: `C:\Users\trans\.cargo\bin`.
- Toolchain: `C:\Users\trans\.rustup\toolchains\stable-x86_64-pc-windows-msvc`.
- Host e alvo: `x86_64-pc-windows-msvc`.
- Toolchain ativa e padrão: `stable-x86_64-pc-windows-msvc`.

O rustup adicionou `C:\Users\trans\.cargo\bin` ao PATH do usuário. A entrada
existente do REA foi preservada. A resolução dos comandos foi verificada após
recarregar, apenas no processo de verificação, os PATHs persistidos de máquina
e usuário. Terminais já abertos podem precisar ser reiniciados para herdá-los.

## Versões verificadas

| Componente | Versão / resultado |
| --- | --- |
| Windows | Windows 11 Professional x64, versão NT 10.0.26200 |
| rustc | 1.99.1 (`58cf8f9c3`, 2026-10-13) |
| Cargo | 1.99.1 (`5f94df478`, 2026-08-27) |
| rustup | 1.29.3 (`edbc831ce`, 2026-11-02) |
| LLVM do rustc | 23.1.1 |
| Git | 2.54.0.windows.1 |
| Visual Studio existente | Community 2026, 18.8.2 |
| MSVC existente | Toolset 14.51.36231; `cl.exe` 19.51.36252.0 |
| Linker MSVC existente | `link.exe` 14.51.36252.0 |
| Windows SDK existente | 10.0.26100.0; headers e bibliotecas x64 conferidos |

O Visual Studio, o MSVC e o SDK não foram reinstalados ou reconfigurados.
O build Cargo para o alvo MSVC funcionou sem configuração manual de linker
ou alteração permanente das variáveis de ambiente do Visual Studio.

Saídas completas de `rustc --version --verbose`, `cargo --version --verbose`,
`rustup --version`, `rustup show` e `rustup component list --installed`:
[MARK_0_toolchain.txt](MARK_0_toolchain.txt).

## Compilação e execução

Criação do programa de validação:

```powershell
cargo new C:\Dev\Arua\validation\mark0-smoke --bin --name arua_mark0_smoke --vcs none
```

O programa usa Rust edition 2024 e somente a biblioteca padrão. Seu código
apenas imprime uma mensagem de confirmação. Na pasta desse programa:

```powershell
cargo build --offline --target x86_64-pc-windows-msvc --verbose
cargo run --offline --target x86_64-pc-windows-msvc
```

Ambos os comandos terminaram com código 0. A execução produziu:

```text
Arua Mark 0: ambiente Rust/MSVC validado.
```

O cabeçalho PE do executável foi lido e confirmou `Machine = 0x8664` (AMD64).

Executável local:
`validation\mark0-smoke\target\x86_64-pc-windows-msvc\debug\arua_mark0_smoke.exe`.

SHA-256 do executável produzido nesta validação:

```text
A0679BC5354B7F0D6A91DD532B89679811F3E35B578543F23114F052B3A9249A
```

Esse hash identifica o arquivo desta execução; não constitui garantia de build
reproduzível em outro caminho ou ambiente.

Logs: [build](MARK_0_build.txt) e [execução](MARK_0_run.txt).

## Preservação do REA

Foram copiados de `C:\Users\trans\OneDrive\Desktop\Arua` para `tools\rea`:

- `Chrome - REA.lnk`.
- `Edge - REA.lnk`.
- `LEIA-ME - REA.txt`.

Os hashes SHA-256 das três cópias conferem com os originais. Os originais
permanecem na pasta do OneDrive. Os atalhos mantêm os destinos e os argumentos
originais, incluindo os perfis dedicados e as portas 9222 e 9223.

A instalação interna continua em `C:\Users\trans\.local\REA`. Nenhum componente
interno do REA foi movido ou reinstalado. A configuração do Codex, a skill REA
e o launcher `rea.cmd` tiveram seus hashes conferidos antes e depois desta
missão, sem alterações. Os resultados estão em
[MARK_0_integrity.json](MARK_0_integrity.json).

A auditoria anterior confirmou REA 6.8.0 conectado ao Codex. Nesta missão não
foram abertos navegadores nem repetidos testes de conexão com perfis fechados;
a disponibilidade desses perfis depende de sua abertura pelos atalhos.

## Git e arquivos gerados

O `.gitignore` cobre diretórios `target`, downloads locais em `.mark0`, estado
local de IDE, arquivos temporários e arquivos comuns do Windows.

Foi confirmado com `git check-ignore` que o executável de validação e o
instalador estão ignorados. O `Cargo.lock` do programa não está ignorado e
pode ser versionado. Não foi criado um repositório Git aninhado no programa.

```text
C:\Dev\Arua\
├── .git\
├── .gitignore
├── .mark0\                         # ignorado pelo Git; downloads/log de instalação
│   ├── rustup-init.exe
│   ├── rustup-init.exe.sha256
│   └── rustup-install.log
├── docs\
│   ├── MARK_0.md
│   ├── MARK_0_toolchain.txt
│   ├── MARK_0_build.txt
│   ├── MARK_0_run.txt
│   └── MARK_0_integrity.json
├── tools\
│   └── rea\
│       ├── Chrome - REA.lnk
│       ├── Edge - REA.lnk
│       └── LEIA-ME - REA.txt
└── validation\
    └── mark0-smoke\
        ├── Cargo.toml
        ├── Cargo.lock
        ├── src\
        │   └── main.rs
        └── target\                 # ignorado pelo Git
```

## Pendências e limites

- Não há bloqueios identificados para compilar e executar programas Rust x64
  com o alvo MSVC neste ambiente.
- Não foram feitos commit, publicação ou configuração de remote Git.
- O perfil Rust instalado é mínimo; rustfmt, Clippy e rust-analyzer não foram
  instalados como componentes. Não são necessários para o teste desta Mark.
- A validação cobre um executável de console em debug. Interface gráfica,
  renderização, rede e demais componentes do navegador ainda não existem.
- Nenhum teste foi executado dentro do OneDrive. Nenhum arquivo existente foi
  apagado ou movido, e nenhuma proteção de segurança do Windows foi desativada.
- Mark I não iniciada. Próximo trabalho depende de nova missão.
