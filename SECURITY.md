# Segurança

## Escopo e uso autorizado

O VBA8 processa arquivos locais não confiáveis. Use somente com autorização do titular. Ele não executa VBA, não torna macros seguras e não deve ser usado para contornar direitos de terceiros.

Não habilite macros para testar a recuperação. Não modifique Trust Center, registro do Office ou permissões do sistema para fazer uma fixture funcionar.

## Medidas implementadas

- Processamento de ZIP/CFB com limites de tamanho, entradas e texto.
- Recusa de duplicatas, caminhos inseguros, entradas criptografadas/symlinks e estados ambíguos.
- Recusa preventiva de assinaturas conhecidas.
- Fonte somente leitura, identidade de destino conferida e publicação sem sobrescrita.
- Interface local com CSP; sem plugins genéricos de shell/filesystem expostos à UI.

Essas medidas reduzem riscos, mas não equivalem a uma auditoria de segurança independente, sandbox de arquivos maliciosos ou garantia de ausência de vulnerabilidades.

## Relatar um problema

Não publique planilhas confidenciais, credenciais, dados pessoais ou macros perigosas em issues. Use o canal privado do mantenedor ou a função **Security → Report a vulnerability** do GitHub, se estiver habilitada. Sem canal privado disponível, abra apenas um pedido de contato sem detalhes exploráveis ou dados sensíveis.

Inclua versão/commit, Windows/Office, erro reproduzível e uma amostra mínima sanitizada somente após combinar um canal seguro. Não execute conteúdo suspeito para reproduzir.

## Distribuição

Não há assinatura de código nem instalador nesta versão. Confira origem e SHA-256 do artefato recebido. O runtime WebView2 é compartilhado e deve permanecer atualizado. Argumentos de debug remoto usados em QA não devem acompanhar a execução de produção.
