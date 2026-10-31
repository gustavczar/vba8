# Dependências e avisos de terceiros

O código próprio pertence a Gustavo Cezar Lourenço. Dependências e materiais de terceiros mantêm seus autores e licenças. Esses avisos não tornam terceiros coautores do aplicativo nem podem ser omitidos por uma declaração de autoria exclusiva do projeto.

## Interface — metadados instalados

| Pacote | Versão local | Licença declarada |
|---|---|---|
| `@fontsource/vt323` | 5.3.0 | OFL-1.1 |
| `@fontsource/ibm-plex-mono` | 5.3.0 | OFL-1.1 |
| `react` | 19.3.1 | MIT |
| `react-dom` | 19.3.1 | MIT |
| `@tauri-apps/api` | 2.12.4 | Apache-2.0 OR MIT |

Textos de licença disponíveis para fontes, React e API Tauri foram preservados em [docs/licenses](docs/licenses). Fontes VT323 e IBM Plex Mono são materiais de terceiros, não criação do mantenedor.

## Núcleo e desktop

Rust utiliza Tauri, tauri-plugin-dialog, cfb, zip, serde, serde_json, thiserror, tempfile, same-file e quick-xml, entre suas dependências diretas. As versões exatas e dependências transitivas são fixadas nos dois Cargo.lock. Consulte as licenças nos pacotes upstream antes de redistribuir binários. A listagem não é uma certificação jurídica ou inventário completo de obrigações transitivas.

## Referências de pesquisa e fixtures

A implementação estudou documentação Microsoft MS-OVBA/MS-CFB e alternativas públicas. Não inclui código baixado dos candidatos pesquisados nem fixtures externas no repositório. As referências técnicas constam em docs/ARCHITECTURE.md; as origens e hashes das amostras usadas em testes opcionais constam em docs/TESTING.md. Licenças das amostras precisam ser preservadas por quem as baixar.

## Ferramentas de desenvolvimento

React, Vite, TypeScript, Tailwind, Vitest, Testing Library e dependências de QA mantêm suas licenças próprias. package-lock.json e Cargo.lock são as fontes de versões, não o texto resumido deste documento.
