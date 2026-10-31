# Testes e limites da evidência

## Suíte padrão

```powershell
python -m unittest discover -s tests -v
npm ci --prefix ui
npm test --prefix ui
npm run build --prefix ui
cargo test --manifest-path core/Cargo.toml --locked
cargo test --manifest-path desktop/Cargo.toml --locked
```

A suíte padrão usa fixtures sintéticas inertes construídas nos testes. Ela não baixa workbooks, abre Excel ou executa macros. O teste de integração externa é ignorado por padrão.

## O que cada camada prova

| Camada | Prova | Não prova |
|---|---|---|
| Core sintético | Parsing, casos negativos, semântica DPB, escopo de alteração, destino preservado | Compatibilidade VBE |
| Core com XLSM real | Processamento de contêineres reais e preservação após patch | Remoção efetiva no editor |
| React com IPC mockado | Estados da UI e contrato das chamadas | Diálogos nativos ou escrita real |
| Tauri real | Integração janela/IPC/diálogos/núcleo | Todas as versões do Windows/Office |
| Excel readonly | Abertura normal, presença VBA, valores e hashes intactos | Desbloqueio, salvamento e reabertura VBE |

## Integração externa opcional

A validação local original usou três fixtures públicas MIT. Não são incluídas no repositório para evitar distribuir planilhas com macros e dados de pesquisa por padrão.

Origens fixadas:

- `jmacadie/unlock_excel`, commit `06ff5e115fbb1f1a27e2c7558bf80c21820ece6e`, `tests/data/xlsm/Locked_with_macro.xlsm`, SHA-256 `a92f7d17ef6e259940529cc5b61422214da889c9b42665fbac14ffd5e802291e`.
- Mesmo repositório/commit, `Locked_with_macro_and_complex_password.xlsm`, SHA-256 `73c1fdf4f7d0effce059cd6a7bbfa4a8463139550a6d7cdebc21f7bdf7acfffe`.
- `petemc89/craXcel`, commit `798f221f754d80e9980497efc797d48376079a3d`, `test-files/locked.xlsm`, SHA-256 `3707a24ffa6220a9031310b838edb1e7ea10c97defe2ee24c5f700e6aab3e60d`.

Baixe apenas como dados inertes, confira os hashes e preserve as licenças upstream. Não abra nem execute suas macros. Crie um `manifest.json` no diretório dessas fixtures:

```json
{"fixtures":[
  {"local_file":"Locked_with_macro.xlsm"},
  {"local_file":"Locked_with_macro_and_complex_password.xlsm"},
  {"local_file":"locked.xlsm"}
]}
```

O formato deve usar os nomes exatos dos arquivos presentes. Defina um controle negativo XLSM genuinamente sem senha e execute:

```powershell
$env:VBA8_FIXTURE_DIR = 'X:\fixtures\public-protected'
$env:VBA8_NEGATIVE_FIXTURE = 'X:\fixtures\unprotected.xlsm'
cargo test --manifest-path core/Cargo.toml --locked -- --include-ignored
```

O teste consome os arquivos locais, gera saídas em temporários e não inicia Office. Os hashes acima documentam a evidência original; o teste Rust compara a origem antes/depois, mas não substitui sua conferência prévia de proveniência.

## Validação manual de release

1. Compilar de checkout limpo com lockfiles.
2. Abrir EXE sem argumentos de debug remoto.
3. Testar abertura/cancelamento, diagnóstico sem senha, autorização, gerar/cancelar e recusa de destino existente.
4. Conferir nome e diretório efetivos do output, inclusive pasta arbitrária e caminhos Unicode.
5. Comparar SHA-256 da origem e conteúdo dos membros ZIP/streams CFB.
6. Em ambiente Office isolado, desabilitar execução de macros e eventos antes de abrir controles.
7. Testar separadamente o ciclo VBE descrito em USAGE.md e registrar versão/build Office.
8. Fechar sem sobrescrever o original e registrar exatamente o que passou.

## Estado atual

A execução inicial passou em 12 testes de núcleo (incluindo a integração externa), 7 de UI, 5 de desktop e 10 auxiliares Windows. A cópia foi gerada pela GUI nativa e verificada independentemente. Excel 16.0 build 20527 abriu original/cópia readonly com macros desabilitadas e valores iguais.

Pendências conhecidas: ciclo VBE completo e certificação independente de destino arbitrário na automação. A contagem padrão do núcleo é 11 aprovados e 1 ignorado sem fixtures externas. Não chamar o workflow de verde antes de consultar a execução remota.
