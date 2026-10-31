# Guia de uso

## Antes de começar

Use apenas planilhas próprias ou autorizadas. Feche o original no Excel e preserve-o. O aplicativo altera uma nova cópia; não use a saída como substituta definitiva antes de conferir os dados e o projeto no Office.

O VBA8 é destinado à proteção de visualização do projeto VBA em `.xlsm`. Senha para abrir o arquivo, proteção de células e estrutura são mecanismos diferentes e não são removidos.

## No aplicativo

1. Abra o executável em um Windows x64 com WebView2.
2. Clique em **Selecionar arquivo**. O aplicativo inspeciona o pacote sem executar VBA.
3. Confira o diagnóstico. A geração só fica disponível se `can_recover` for verdadeiro.
4. Confirme sua autorização.
5. Clique em **Gerar cópia**. Escolha um novo nome `.xlsm` e destino.
6. Se já existir um arquivo nesse destino, escolha outro nome: o núcleo recusa sobrescrita, mesmo que o diálogo do sistema ofereça confirmação.
7. Aguarde a mensagem de cópia gerada e verificada.

Cancelar a abertura limpa a seleção anterior. Cancelar o salvamento não gera sucesso e permite tentar novamente.

## Depois da cópia: Excel e VBE

O método `DPB → DPx` não revela nem substitui a senha por um valor conhecido. A chave inválida pode provocar avisos no editor. Estes passos são uma orientação de compatibilidade, não uma garantia para todas as versões:

1. Abra somente a cópia e apenas se confiar em sua origem. Não habilite macros para executar o procedimento.
2. Pressione **Alt + F11**.
3. Se houver aviso de chave DPx inválida com uma opção de continuar, prossiga somente na cópia. Se o editor não permitir acesso às propriedades, pare.
4. Abra **Ferramentas → Propriedades do VBAProject → Proteção**. Em inglês: **Tools → VBAProject Properties → Protection**.
5. Se as propriedades estiverem acessíveis, defina uma senha temporária conhecida, confirme e salve.
6. Feche completamente o Excel, reabra a cópia e use essa senha temporária.
7. Volte às propriedades, desmarque **Bloquear projeto para exibição / Lock project for viewing**, limpe a senha, salve, feche e reabra.
8. Confira módulos, formulários, referências, dados e comportamento esperado antes de adotar a cópia.

Algumas versões podem aceitar uma senha qualquer durante a recuperação, mas isso não é universal. Não tente forçar uma senha arbitrária se seu Office rejeitar o método. Volte ao original.

## Diagnósticos

| Situação | Significado |
|---|---|
| Proteção identificada | Campo DPB reconhecido com dados de senha; pode gerar cópia |
| Sem senha | Sentinela normativo de ausência de senha decodificado |
| Já processado | Marcador DPx encontrado; não reaplica o patch |
| Nenhum VBA | Pacote não contém o projeto esperado |
| PROJECT ausente | CFB sem stream raiz PROJECT |
| Desconhecido/erro | Estrutura não reconhecida ou insegura; não presumir ausência de senha |
| Assinatura detectada | Recusa preventiva para não modificar confiança/assinatura |
| ZIP ilegível | Pode ser arquivo inválido, formato não suportado ou documento com criptografia de abertura |

## Recuperação e rollback

Se a cópia apresentar qualquer problema, feche-a sem substituir o original. Preserve a saída e o erro para diagnóstico. Não envie planilhas confidenciais em uma issue pública. O rollback é voltar ao arquivo de origem, que a recuperação não grava.

## Estado de compatibilidade

A geração de cópias e preservação de conteúdo foram exercitadas em arquivos reais. Abertura readonly no Excel foi conferida. O ciclo completo de desbloqueio/salvamento/reabertura no VBE ainda não foi validado. Consulte [TESTING.md](TESTING.md).
