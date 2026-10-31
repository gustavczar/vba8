# Desenvolvimento

## Ambiente Windows

Instale Rust estável (edição 2024), Node LTS compatível com Vite 7, Python 3.10+, Visual Studio Build Tools com C++/Windows SDK e WebView2.

```powershell
npm ci --prefix ui
python -m unittest discover -s tests -v
python build.py
```

`build.py` executa testes UI, build frontend, testes core/desktop e build release. Ele não instala dependências nem altera PATH global. O frontend precisa ser compilado antes do desktop porque `tauri::generate_context!` inclui `ui/dist` no executável.

Para instalações portáteis:

```powershell
$env:VBA8_CARGO = 'X:\Rust\cargo\bin\cargo.exe'
$env:VBA8_NPM = 'X:\Node\npm.cmd'
$env:CARGO_HOME = 'X:\Rust\cargo'
$env:RUSTUP_HOME = 'X:\Rust\rustup'
python build.py
```

Os exemplos usam diretórios ilustrativos; não há caminho de usuário obrigatório no script de build.

## Comandos diretos

```powershell
npm test --prefix ui
npm run build --prefix ui
cargo test --manifest-path core/Cargo.toml --locked
cargo fmt --manifest-path core/Cargo.toml -- --check
cargo clippy --manifest-path core/Cargo.toml --locked -- -D warnings
cargo test --manifest-path desktop/Cargo.toml --locked
cargo build --release --manifest-path desktop/Cargo.toml --locked
```

Para visualizar somente a interface: `npm run dev --prefix ui`. A visualização no navegador não possui o backend Tauri: selecionar/salvar informa que é necessário abrir o app desktop, em vez de simular sucesso. Vite e o `devUrl` do Tauri usam porta 1420. O build empacotado usa `frontendDist` e não depende desse servidor.

## CLI

```powershell
cargo run --manifest-path core/Cargo.toml -- --inspect arquivo.xlsm
cargo run --manifest-path core/Cargo.toml -- --recover arquivo.xlsm copia.xlsm
```

CLI retorna JSON; erros usam código de saída não zero. Nunca execute o CLI sobre planilhas confidenciais em logs públicos.

## Estrutura de mudanças

- Regras de formato e preservação pertencem ao núcleo, não à UI.
- Mudanças de interface precisam cobrir vazio, loading, cancelamento, sucesso e erro.
- Nunca declare desbloqueio no Excel a partir de um teste sintético ou da abertura de um workbook.
- Não embuta caminhos locais, fixtures privadas, saídas de QA ou credenciais em commits.
- Mantenha os lockfiles Rust/npm versionados.

## Distribuição

O build atual produz um EXE, sem instalador e sem assinatura de código. Não publica automaticamente releases. Uma publicação deve registrar o hash SHA-256 do binário e informar dependência WebView2 e limites de compatibilidade. Não altere as políticas de macros do Office para facilitar testes.
