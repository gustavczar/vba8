# Histórico de alterações

## 0.1.0 — versão inicial para revisão

### Funcionalidades

- Núcleo Rust para inspeção e geração conservadora de cópia XLSM.
- Decodificação semântica DPB para evitar classificar projeto sem senha como protegido.
- Patch de um byte restrito ao stream raiz PROJECT.
- Gravação temporária, verificação de preservação e recusa de sobrescrita.
- Interface 8-bits em português com seleção/salvamento nativos e instruções integradas.
- Suítes de núcleo, UI e adaptador desktop.

### Documentação e publicação

- README, guias de uso, arquitetura, desenvolvimento, testes e segurança.
- Build por PATH ou overrides explícitos, sem diretórios pessoais obrigatórios.
- Workflow Windows para testar e compilar o aplicativo.
- Fixtures externas e artefatos locais excluídos do código versionado.

### Limites conhecidos

- Ciclo completo de desbloqueio/salvamento/reabertura VBE ainda não validado.
- Sem garantia para toda versão de Office, sem instalador e sem assinatura de código.
- Alguns erros técnicos permanecem em inglês.
